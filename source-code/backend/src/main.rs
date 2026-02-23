use anyhow::{Context as _, Result};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use error::{output_error, ErrorCode};
use std::env;
use std::fs;
use std::path::Path;
use std::process::exit;
use verify::verify;
use state::{load_state, save_state, update_state};
use sandbox::setup_sandbox;

mod error;
mod manifest;
mod sandbox;
mod state;
mod verify;

const PUBLIC_KEY_BYTES: [u8; 32] = [0u8; 32];

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        output_error(ErrorCode::InvalidArgs, "Invalid arguments");
    }
    let command = &args[0];
    match command.as_str() {
        "install" => {
            if args.len() < 5 {
                output_error(ErrorCode::InvalidArgs, "Usage: backend install <package> <version> <path> <checksum>");
            }
            if let Err(e) = install(&args[1], &args[2], &args[3], &args[4]) {
                output_error(ErrorCode::InstallFailed, &format!("Install failed: {}", e));
            }
        }
        "remove" => {
            if args.len() < 4 {
                output_error(ErrorCode::InvalidArgs, "Usage: backend remove <package> <version> <path>");
            }
            if let Err(e) = remove(&args[1], &args[2], &args[3]) {
                output_error(ErrorCode::RemoveFailed, &format!("Remove failed: {}", e));
            }
        }
        "verify" => {
            if args.len() < 3 {
                output_error(ErrorCode::InvalidArgs, "Usage: backend verify <path> <checksum>");
            }
            if let Err(e) = verify(&args[1], &args[2]) {
                output_error(ErrorCode::VerificationFailed, &format!("Verification failed: {}", e));
            }
            println!("{}", serde_json::json!({ "success": true }));
        }
        "verify-signature" => {
            if args.len() < 3 {
                output_error(ErrorCode::InvalidArgs, "Usage: backend verify-signature <path> <signature>");
            }
            if let Err(e) = verify_signature(&args[1], &args[2]) {
                output_error(ErrorCode::VerificationFailed, &format!("Signature verification failed: {}", e));
            }
            println!("{}", serde_json::json!({ "success": true }));
        }
        "list-installed" => {
            if let Err(e) = list_installed() {
                output_error(ErrorCode::UnknownCommand, &format!("List installed failed: {}", e));
            }
        }
        "sandbox-test" => {
            if args.len() < 2 {
                output_error(ErrorCode::InvalidArgs, "Usage: backend sandbox-test <path>");
            }
            if let Err(e) = sandbox_test(&args[1]) {
                output_error(ErrorCode::InstallFailed, &format!("Sandbox test failed: {}", e));
            }
            println!("{}", serde_json::json!({ "success": true }));
        }
        "run" => {
            if args.len() < 3 { exit(1); }
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
            let new_p = Path::new(&tmp_path).join(entry.file_name());
            fs::rename(&old_p, &new_p).context("Move failed")?;
        }
        fs::remove_dir(&contents_path).context("Remove contents dir failed")?;
    }
    let manifest = manifest::Manifest::load_info(&tmp_path)?;
    verify(&tmp_path, checksum)?;
    setup_sandbox(&tmp_path, &manifest, true, None, vec![], false).context("Sandbox setup failed")?;
    let path_p = Path::new(path);
    let path_old = format!("{}.old", path);
    let mut backed_up = false;
    let res = (|| -> Result<()> {
        if path_p.exists() {
            fs::rename(path, &path_old)?;
            backed_up = true;
        }
        fs::rename(&tmp_path, path).context("Rename failed")?;
        update_state(package_name, version, checksum)?;
        Ok(())
    })();
    if let Err(e) = res {
        if backed_up {
            let _ = fs::remove_dir_all(path);
            fs::rename(&path_old, path).context("Rollback failed")?;
        } else {
            let _ = fs::remove_dir_all(path);
        }
        return Err(e);
    }
    if backed_up {
        fs::remove_dir_all(&path_old).context("Remove backup failed")?;
    }
    println!("{}", serde_json::json!({ "success": true, "package_name": package_name }));
    Ok(())
}

fn remove(package_name: &str, version: &str, path: &str) -> Result<()> {
    let manifest = manifest::Manifest::load_info(path)?;
    for bin in &manifest.bins {
        let _ = fs::remove_file(format!("/usr/bin/{}", bin));
    }
    fs::remove_dir_all(path).context("Delete tree failed")?;
    let mut state = load_state()?;
    if let Some(vers) = state.packages.get_mut(package_name) {
        vers.remove(version);
        if vers.is_empty() { state.packages.remove(package_name); }
    }
    save_state(&state)?;
    println!("{}", serde_json::json!({ "success": true, "package_name": package_name }));
    Ok(())
}

fn verify_signature(path: &str, signature_base64: &str) -> Result<()> {
    let data = fs::read(path).context("Failed to read file")?;
    let sig_bytes = general_purpose::STANDARD.decode(signature_base64).context("Failed to decode signature")?;
    let signature = Signature::try_from(sig_bytes.as_slice()).context("Invalid signature length")?;
    let verifying_key = VerifyingKey::from_bytes(&PUBLIC_KEY_BYTES).context("Invalid public key")?;
    verifying_key.verify(&data, &signature).context("Signature verification failed")?;
    Ok(())
}

fn list_installed() -> Result<()> {
    let state = load_state()?;
    println!("{}", serde_json::to_string(&state)?);
    Ok(())
}

fn sandbox_test(path: &str) -> Result<()> {
    let manifest = manifest::Manifest::load_info(path)?;
    setup_sandbox(path, &manifest, false, None, vec![], true)
}

fn run(args: &[String]) -> Result<()> {
    let package_name = &args[0];
    let bin = &args[1];
    let extra_args = args[2..].to_vec();
    let path = format!("{}{}/current", crate::sandbox::STORE_PATH, package_name);
    let manifest = manifest::Manifest::load_info(&path)?;
    setup_sandbox(&path, &manifest, false, Some(bin), extra_args, false)?;
    Ok(())
}
