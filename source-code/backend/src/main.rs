use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use landlock::{
    path_beneath_rules, Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreated, RulesetCreatedAttr, ABI,
};
use nix::mount::{mount, umount2, MntFlags, MsFlags};
use nix::sched::{unshare, CloneFlags};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{chdir, execve, fork, pivot_root, ForkResult, Gid, Uid};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::ffi::{CStr, CString};
use std::fs::{self, create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::exit;

const STORE_PATH: &str = "/usr/lib/HackerOS/hpm/store/";
const STATE_PATH: &str = "/var/lib/hpm/state.json";

#[derive(Debug)]
struct Manifest {
    name: String,
    version: String,
    authors: String,
    license: String,
    summary: String,
    long: String,
    system_specs: IndexMap<String, String>,
    deps: IndexMap<String, String>,
    bins: Vec<String>,
    sandbox: Sandbox,
    install_commands: Vec<String>,
}

#[derive(Debug)]
struct Sandbox {
    network: bool,
    filesystem: Vec<String>,
    gui: bool,
    dev: bool,
}

impl Manifest {
    fn load_info(path: &str) -> Result<Manifest> {
        let info_path = format!("{}/info.hk", path);
        let mut config = hk_parser::load_hk_file(&info_path)
            .map_err(|e| anyhow!("Failed to load info.hk: {}", e))?;
        hk_parser::resolve_interpolations(&mut config)
            .map_err(|e| anyhow!("Failed to resolve interpolations: {}", e))?;

        let metadata = config
            .get("metadata")
            .ok_or(anyhow!("Missing [metadata] section"))?
            .as_map()
            .map_err(|_| anyhow!("Invalid metadata"))?;

        let name = metadata
            .get("name")
            .ok_or(anyhow!("Missing name"))?
            .as_string()
            .map_err(|_| anyhow!("Invalid name"))?;

        let version = metadata
            .get("version")
            .ok_or(anyhow!("Missing version"))?
            .as_string()
            .map_err(|_| anyhow!("Invalid version"))?;

        let authors = metadata
            .get("authors")
            .ok_or(anyhow!("Missing authors"))?
            .as_string()
            .map_err(|_| anyhow!("Invalid authors"))?;

        let license = metadata
            .get("license")
            .ok_or(anyhow!("Missing license"))?
            .as_string()
            .map_err(|_| anyhow!("Invalid license"))?;

        let description = config.get("description").and_then(|v| v.as_map().ok());

        let summary = description
            .and_then(|d| d.get("summary"))
            .and_then(|v| v.as_string().ok())
            .unwrap_or_default();

        let long = description
            .and_then(|d| d.get("long"))
            .and_then(|v| v.as_string().ok())
            .unwrap_or_default();

        let specs = config.get("specs").and_then(|v| v.as_map().ok());

        let mut system_specs = IndexMap::new();
        if let Some(s) = specs {
            for (k, v) in s {
                if k != "dependencies" {
                    system_specs
                        .insert(k.clone(), v.as_string().map_err(|_| anyhow!("Invalid spec value"))?);
                }
            }
        }

        let deps = if let Some(d) = specs
            .and_then(|s| s.get("dependencies"))
            .and_then(|v| v.as_map().ok())
        {
            let mut m = IndexMap::new();
            for (k, v) in d {
                m.insert(k.clone(), v.as_string().map_err(|_| anyhow!("Invalid dep value"))?);
            }
            m
        } else {
            IndexMap::new()
        };

        let bins_map = metadata.get("bins").and_then(|v| v.as_map().ok());
        let mut bins = Vec::new();
        if let Some(bm) = bins_map {
            for (k, v) in bm {
                if v.as_string().map_err(|_| anyhow!("Invalid bin value"))? == "" {
                    bins.push(k.clone());
                }
            }
        }

        let sandbox_sec = config
            .get("sandbox")
            .ok_or(anyhow!("Missing [sandbox] section"))?
            .as_map()
            .map_err(|_| anyhow!("Invalid sandbox"))?;

        let network = sandbox_sec
            .get("network")
            .and_then(|v| v.as_bool().ok())
            .unwrap_or(false);
        let gui = sandbox_sec.get("gui").and_then(|v| v.as_bool().ok()).unwrap_or(false);
        let dev = sandbox_sec.get("dev").and_then(|v| v.as_bool().ok()).unwrap_or(false);

        let fs_map = sandbox_sec.get("filesystem").and_then(|v| v.as_map().ok());
        let mut filesystem = Vec::new();
        if let Some(fm) = fs_map {
            for (k, v) in fm {
                if v.as_string().map_err(|_| anyhow!("Invalid fs value"))? == "" {
                    filesystem.push(k.clone());
                }
            }
        }

        let install_sec = config.get("install").and_then(|v| v.as_map().ok());
        let mut install_commands = Vec::new();
        if let Some(is) = install_sec {
            if let Some(cmds) = is.get("commands").and_then(|v| v.as_map().ok()) {
                for (k, v) in cmds {
                    if v.as_string().map_err(|_| anyhow!("Invalid cmd value"))? == "" {
                        install_commands.push(k.clone());
                    }
                }
            }
        }

        Ok(Manifest {
            name,
            version,
            authors,
            license,
            summary,
            long,
            system_specs,
            deps,
            bins,
            sandbox: Sandbox {
                network,
                filesystem,
                gui,
                dev,
            },
            install_commands,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct State {
    packages: HashMap<String, HashMap<String, String>>,
}

#[derive(Serialize)]
struct ErrorPayload {
    err: ErrorInner,
}

#[derive(Serialize)]
struct ErrorInner {
    code: i32,
    message: String,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum ErrorCode {
    Success = 0,
    InvalidArgs = 1,
    PackageNotFound = 2,    DependencyCycle = 3,
    InstallFailed = 4,
    RemoveFailed = 5,
    VerificationFailed = 6,
    UnknownCommand = 99,
}

fn output_error(code: ErrorCode, msg: &str) {
    let payload = ErrorPayload {
        err: ErrorInner {
            code: code as i32,
            message: msg.to_string(),
        },
    };
    let json = serde_json::to_string(&payload).expect("JSON marshal failed");
    eprintln!("{}", json);
    exit(code as i32);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        output_error(ErrorCode::InvalidArgs, "Invalid arguments");
    }

    let command = &args[0];
    match command.as_str() {
        "install" => {
            if args.len() < 5 {
                output_error(
                    ErrorCode::InvalidArgs,
                    "Usage: backend install <package> <version> <path> <checksum>",
                );
            }
            let package_name = &args[1];
            let version = &args[2];
            let path = &args[3];
            let checksum = &args[4];
            if let Err(e) = install(package_name, version, path, checksum) {
                output_error(ErrorCode::InstallFailed, &format!("Install failed: {}", e));
            }
        }
        "remove" => {
            if args.len() < 4 {
                output_error(
                    ErrorCode::InvalidArgs,
                    "Usage: backend remove <package> <version> <path>",
                );
            }
            let package_name = &args[1];
            let version = &args[2];
            let path = &args[3];
            if let Err(e) = remove(package_name, version, path) {
                output_error(ErrorCode::RemoveFailed, &format!("Remove failed: {}", e));
            }
        }
        "verify" => {
            if args.len() < 3 {
                output_error(
                    ErrorCode::InvalidArgs,
                    "Usage: backend verify <path> <checksum>",
                );
            }
            let path = &args[1];
            let checksum = &args[2];
            if let Err(e) = verify(path, checksum) {
                output_error(
                    ErrorCode::VerificationFailed,
                    &format!("Verification failed: {}", e),
                );
            }
            let payload = serde_json::json!({ "success": true });
            println!("{}", payload);
        }
        "run" => {
            if args.len() < 3 {
                exit(1);
            }
            if let Err(e) = run(&args[1..]) {
                eprintln!("Run failed: {}", e);
                exit(1);
            }
        }
        _ => output_error(ErrorCode::UnknownCommand, "Unknown command"),
    }
}

fn install(package_name: &str, version: &str, path: &str, checksum: &str) -> Result<()> {
    let tmp_path = format!("{}.tmp", path);
    fs::create_dir_all(&tmp_path).context("Failed to create tmp directory")?;

    let contents_path = format!("{}/contents", &tmp_path);
    if Path::new(&contents_path).exists() {
        for entry in fs::read_dir(&contents_path)? {
            let entry = entry?;
            let old_p = entry.path();
            let file_name = entry.file_name();
            let new_p = Path::new(&tmp_path).join(file_name);
            fs::rename(&old_p, &new_p).context("Move failed")?;
        }
        fs::remove_dir(&contents_path).context("Remove contents dir failed")?;
    }

    let manifest = Manifest::load_info(&tmp_path)?;

    if !manifest.deps.is_empty() {
        for (dep, req) in &manifest.deps {
            eprintln!("Dependency: {} {}", dep, req);
        }
    }

    setup_sandbox(&tmp_path, &manifest, true, None, vec![]).context("Sandbox setup failed")?;

    verify(&tmp_path, checksum)?;

    fs::rename(&tmp_path, path).context("Rename failed")?;

    update_state(package_name, version, checksum)?;

    let payload = serde_json::json!({ "success": true, "package_name": package_name });
    println!("{}", payload);

    Ok(())
}

fn remove(package_name: &str, version: &str, path: &str) -> Result<()> {
    let manifest = Manifest::load_info(path)?;

    for bin in &manifest.bins {
        let bin_path = format!("/usr/bin/{}", bin);
        let _ = fs::remove_file(&bin_path);
    }

    fs::remove_dir_all(path).context("Delete tree failed")?;

    let mut state = load_state()?;
    if let Some(vers) = state.packages.get_mut(package_name) {
        vers.remove(version);
        if vers.is_empty() {
            state.packages.remove(package_name);
        }
    }
    save_state(&state)?;

    let payload = serde_json::json!({ "success": true, "package_name": package_name });
    println!("{}", payload);

    Ok(())
}

fn verify(path: &str, checksum: &str) -> Result<()> {
    let info_path = format!("{}/info.hk", path);
    let data = fs::read(&info_path).context("Failed to read info.hk for verify")?;

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hasher.finalize();
    let computed = hex::encode(hash);

    if computed != checksum {
        return Err(anyhow!("Checksum mismatch"));
    }

    Ok(())
}

fn setup_sandbox(
    path: &str,
    manifest: &Manifest,
    is_install: bool,
    bin: Option<&str>,
    extra_args: Vec<String>,
) -> Result<()> {
    let display = env::var("DISPLAY").ok();

    match unsafe { fork()? } {
        ForkResult::Parent { child, .. } => {
            let status = waitpid(child, Some(WaitPidFlag::empty()))?;
            if let WaitStatus::Exited(_, code) = status {
                if code != 0 {
                    return Err(anyhow!("Sandbox command failed with code {}", code));
                }
            } else {
                return Err(anyhow!("Sandbox failed"));
            }
            Ok(())
        }
        ForkResult::Child => {
            // Unshare namespaces
            let mut flags = CloneFlags::CLONE_NEWUSER
                | CloneFlags::CLONE_NEWNS
                | CloneFlags::CLONE_NEWUTS
                | CloneFlags::CLONE_NEWPID
                | CloneFlags::CLONE_NEWCGROUP;
            if !manifest.sandbox.network {
                flags |= CloneFlags::CLONE_NEWNET;
            }
            if !manifest.sandbox.gui {
                flags |= CloneFlags::CLONE_NEWIPC;
            }
            unshare(flags).context("Unshare failed")?;

            // Make mounts private
            mount(
                None::<&str>,
                "/",
                None::<&str>,
                MsFlags::MS_PRIVATE | MsFlags::MS_REC,
                None::<&str>,
            )?;

            // Set up user mapping
            let uid = Uid::current();
            let gid = Gid::current();
            let mut uid_map = File::create("/proc/self/uid_map").context("Open uid_map failed")?;
            writeln!(uid_map, "0 {} 1", uid).context("Write uid_map failed")?;
            let mut setgroups =
                File::create("/proc/self/setgroups").context("Open setgroups failed")?;
            writeln!(setgroups, "deny").context("Write setgroups failed")?;
            let mut gid_map = File::create("/proc/self/gid_map").context("Open gid_map failed")?;
            writeln!(gid_map, "0 {} 1", gid).context("Write gid_map failed")?;

            // Create new root
            let new_root_str = format!("/tmp/hpm_newroot_{}", nix::unistd::getpid());
            let new_root = PathBuf::from(&new_root_str);
            create_dir_all(&new_root)?;
            mount(
                Some("tmpfs"),
                new_root_str.as_str(),
                Some("tmpfs"),
                MsFlags::empty(),
                None::<&str>,
            )?;

            // Mount RO binds
            let ro_paths = vec!["/usr", "/lib", "/lib64", "/bin", "/etc"];
            for p in ro_paths {
                let target = new_root.join(p.trim_start_matches('/'));
                create_dir_all(&target)?;
                mount(
                    Some(p),
                    target.to_str().unwrap(),
                    None::<&str>,
                    MsFlags::MS_BIND | MsFlags::MS_REC | MsFlags::MS_RDONLY,
                    None::<&str>,
                )?;
            }

            // Mount /app
            let app_path = new_root.join("app");
            create_dir_all(&app_path)?;
            mount(
                Some(path),
                app_path.to_str().unwrap(),
                None::<&str>,
                MsFlags::MS_BIND | MsFlags::MS_REC,
                None::<&str>,
            )?;

            // Mount /tmp as tmpfs
            let tmp_path = new_root.join("tmp");
            create_dir_all(&tmp_path)?;
            mount(
                Some("tmpfs"),
                tmp_path.to_str().unwrap(),
                Some("tmpfs"),
                MsFlags::empty(),
                None::<&str>,
            )?;

            // Mount GUI if needed
            if manifest.sandbox.gui {
                let x11_path = tmp_path.join(".X11-unix");
                create_dir_all(&x11_path)?;
                mount(
                    Some("/tmp/.X11-unix"),
                    x11_path.to_str().unwrap(),
                    None::<&str>,
                    MsFlags::MS_BIND | MsFlags::MS_REC,
                    None::<&str>,
                )?;
                if let Some(d) = display {
                    env::set_var("DISPLAY", d);
                }
            }

            // Mount dev if needed
            if manifest.sandbox.dev {
                let dev_path = new_root.join("dev");
                create_dir_all(&dev_path)?;
                mount(
                    Some("/dev"),
                    dev_path.to_str().unwrap(),
                    None::<&str>,
                    MsFlags::MS_BIND | MsFlags::MS_REC,
                    None::<&str>,
                )?;
            }

            // Mount extra filesystems
            for fs_p in &manifest.sandbox.filesystem {
                let target_path = fs_p.trim_start_matches('/');
                let target = new_root.join(target_path);
                if let Some(parent) = target.parent() {
                    create_dir_all(parent)?;
                }
                mount(
                    Some(fs_p.as_str()),
                    target.to_str().unwrap(),
                    None::<&str>,
                    MsFlags::MS_BIND | MsFlags::MS_REC,
                    None::<&str>,
                )?;
            }

            // Mount proc
            let proc_path = new_root.join("proc");
            create_dir_all(&proc_path)?;
            mount(
                Some("proc"),
                proc_path.to_str().unwrap(),
                Some("proc"),
                MsFlags::empty(),
                None::<&str>,
            )?;

            // Mount sys
            let sys_path = new_root.join("sys");
            create_dir_all(&sys_path)?;
            mount(
                Some("sysfs"),
                sys_path.to_str().unwrap(),
                Some("sysfs"),
                MsFlags::empty(),
                None::<&str>,
            )?;

            // Pivot root
            chdir(&new_root)?;
            let old_root_rel = "old_root";
            create_dir_all(old_root_rel)?;
            pivot_root(".", old_root_rel)?;
            chdir("/")?;
            umount2("/old_root", MntFlags::MNT_DETACH)?;

            // Chdir to /app
            chdir("/app")?;

            // Landlock
            let abi = ABI::V1;
            let ruleset = Ruleset::default().handle_access(AccessFs::from_all(abi))?;
            let mut attr: RulesetCreated = ruleset.create()?;

            let ro_access = AccessFs::Execute | AccessFs::ReadFile | AccessFs::ReadDir;
            // FIXED: Chained usage of add_rules/add_rule as they consume self and return Result<Self>
            attr = attr.add_rules(path_beneath_rules(
                &["/usr", "/lib", "/lib64", "/bin", "/etc"],
                ro_access,
            ))?;

            let proc_sys_access = AccessFs::ReadFile | AccessFs::ReadDir;
            attr = attr.add_rules(path_beneath_rules(&["/proc", "/sys"], proc_sys_access))?;

            attr = attr.add_rule(PathBeneath::new(
                PathFd::new("/app")?,
                AccessFs::from_all(abi),
            ))?;

            if manifest.sandbox.dev {
                attr = attr.add_rule(PathBeneath::new(
                    PathFd::new("/dev")?,
                    AccessFs::from_all(abi),
                ))?;
            }

            attr = attr.add_rule(PathBeneath::new(
                PathFd::new("/tmp")?,
                AccessFs::from_all(abi),
            ))?;

            for fs_p in &manifest.sandbox.filesystem {
                attr = attr.add_rule(PathBeneath::new(
                    PathFd::new(fs_p)?,
                    AccessFs::from_all(abi),
                ))?;
            }

            attr.restrict_self()?;

            // No seccomp needed, as unshare_net handles network

            // Exec
            let (cmd, args_c): (CString, Vec<CString>) = if is_install {
                let install_cmd = if manifest.install_commands.is_empty() {
                    "echo 'Isolated install complete'".to_string()
                } else {
                    manifest.install_commands.join(" && ")
                };
                (
                    CString::new("/bin/sh")?,
                    vec![CString::new("-c")?, CString::new(install_cmd)?],
                )
            } else {
                let bin = bin.expect("Bin required for run");
                let bin_path = format!("/app/{}", bin);
                let mut a = vec![CString::new(bin_path.as_str())?];
                for arg in extra_args {
                    a.push(CString::new(arg)?);
                }
                (CString::new(bin_path)?, a)
            };

            execve(
                &cmd,
                &args_c.iter().map(|c| c.as_c_str()).collect::<Vec<_>>(),
                &[] as &[&CStr],
            )?;
            unreachable!()
        }
    }
}

fn run(args: &[String]) -> Result<()> {
    let package_name = &args[0];
    let bin = &args[1];
    let extra_args = args[2..].to_vec();

    let path = format!("{}{}/current", STORE_PATH, package_name);

    let manifest = Manifest::load_info(&path)?;

    setup_sandbox(&path, &manifest, false, Some(bin), extra_args)?;

    Ok(())
}

fn load_state() -> Result<State> {
    if !Path::new(STATE_PATH).exists() {
        return Ok(State::default());
    }
    let data = fs::read(STATE_PATH)?;
    serde_json::from_slice(&data).map_err(Into::into)
}

fn save_state(state: &State) -> Result<()> {
    let data = serde_json::to_vec(state)?;
    fs::write(STATE_PATH, data)?;
    Ok(())
}

fn update_state(package_name: &str, version: &str, checksum: &str) -> Result<()> {
    let mut state = load_state()?;
    state
        .packages
        .entry(package_name.to_string())
        .or_insert_with(HashMap::new)
        .insert(version.to_string(), checksum.to_string());
    save_state(&state)
}
