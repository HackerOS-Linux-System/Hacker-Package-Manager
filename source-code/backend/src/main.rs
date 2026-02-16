package main

import "core:fmt"
import "core:os"
import "core:mem"
import "core:strings"
import "core:encoding/json"
import "core:crypto/sha2"
import "core:sys/linux"
import "core:strconv"

Manifest :: struct {
    name: string,
    version: string,
    authors: string,
    license: string,
    summary: string,
    long: string,
    system_specs: map[string]string,
    deps: map[string]string,
    bins: [dynamic]string,
    sandbox: struct {
        network: bool,
        filesystem: [dynamic]string,
        gui: bool,
        dev: bool,
    },
    install_commands: [dynamic]string,
}

deinit_manifest :: proc(self: ^Manifest, allocator: mem.Allocator) {
    delete(self.name, allocator)
    delete(self.version, allocator)
    delete(self.authors, allocator)
    delete(self.license, allocator)
    delete(self.summary, allocator)
    delete(self.long, allocator)
    for key, value in self.system_specs {
        delete(key, allocator)
        delete(value, allocator)
    }
    delete(self.system_specs)
    for key, value in self.deps {
        delete(key, allocator)
        delete(value, allocator)
    }
    delete(self.deps)
    for bin in self.bins {
        delete(bin, allocator)
    }
    delete(self.bins)
    for path in self.sandbox.filesystem {
        delete(path, allocator)
    }
    delete(self.sandbox.filesystem)
    for cmd in self.install_commands {
        delete(cmd, allocator)
    }
    delete(self.install_commands)
}

PackageInfo :: string // checksum

State :: struct {
    packages: map[string]map[string]PackageInfo,
}

deinit_state :: proc(self: ^State, allocator: mem.Allocator) {
    for pkg, vers in self.packages {
        for ver, checksum in vers {
            delete(ver, allocator)
            delete(checksum, allocator)
        }
        delete(vers)
        delete(pkg, allocator)
    }
    delete(self.packages)
}

ErrorCode :: enum i32 {
    Success = 0,
    InvalidArgs = 1,
    PackageNotFound = 2,
    DependencyCycle = 3,
    InstallFailed = 4,
    RemoveFailed = 5,
    VerificationFailed = 6,
    UnknownCommand = 99,
}

STORE_PATH :: "/usr/lib/HackerOS/hpm/store/"

main :: proc() {
    arena: mem.Arena
    backing := make([]u8, 8 * mem.Megabyte)
    mem.arena_init(&arena, backing)
    defer delete(backing)
    allocator := mem.arena_allocator(&arena)
    context.allocator = allocator
    args := os.args[1:]
    if len(args) < 1 {
        output_error(allocator, .InvalidArgs, "Invalid arguments")
        return
    }
    command := args[0]
    switch command {
        case "install":
            if len(args) < 5 {
                output_error(allocator, .InvalidArgs, "Usage: backend install <package> <version> <path> <checksum>")
                return
            }
            package_name := args[1]
            version := args[2]
            path := args[3]
            checksum := args[4]
            install(allocator, package_name, version, path, checksum)
        case "remove":
            if len(args) < 4 {
                output_error(allocator, .InvalidArgs, "Usage: backend remove <package> <version> <path>")
                return
            }
            package_name := args[1]
            version := args[2]
            path := args[3]
            remove(allocator, package_name, version, path)
        case "verify":
            if len(args) < 3 {
                output_error(allocator, .InvalidArgs, "Usage: backend verify <path> <checksum>")
                return
            }
            path := args[1]
            checksum := args[2]
            verify(allocator, path, checksum)
            payload := struct { success: bool }{true}
            output, merr := json.marshal(payload)
            if merr != nil {
                panic("JSON marshal failed")
            }
            defer delete(output)
            os.write(os.stdout, output)
        case "run":
            if len(args) < 3 {
                os.exit(1)
            }
            run(allocator, args[1:])
        case:
            output_error(allocator, .UnknownCommand, "Unknown command")
    }
}

output_error :: proc(allocator: mem.Allocator, code: ErrorCode, msg: string) {
    payload := struct {
        err: struct {
            code: i32,
            message: string,
        },
    }{{code = i32(code), message = msg}}
    output, merr := json.marshal(payload)
    if merr != nil {
        panic("JSON marshal failed")
    }
    os.write(os.stderr, output)
    delete(output)
    os.exit(int(i32(code)))
}

load_info :: proc(allocator: mem.Allocator, path: string) -> Manifest {
    info_path := fmt.aprintf("{}/info.hk", path)
    defer delete(info_path)
    data, ok := os.read_entire_file(info_path)
    if !ok {
        panic("Failed to read info.hk")
    }
    defer delete(data)
    lines := strings.split_lines(string(data))
    defer delete(lines, allocator)
    manifest: Manifest
    current_section := ""
    last_key := ""
    for line in lines {
        l := strings.trim_space(line)
        if l == "" || strings.has_prefix(l, "!") {
            continue
        }
        if strings.has_prefix(l, "[") {
            end_idx := strings.index(l, "]")
            if end_idx == -1 { continue }
            current_section = l[1:end_idx]
            last_key = ""
            continue
        }
        if strings.has_prefix(l, "->") {
            l = strings.trim_space(l[2:])
            idx := strings.index(l, "=>")
            if idx == -1 {
                last_key = l
                continue
            }
            key := strings.trim_space(l[:idx])
            value := strings.trim_space(l[idx+2:])
            last_key = ""
            set_value(&manifest, current_section, key, value, allocator)
        } else if strings.has_prefix(l, "-->") {
            l = strings.trim_space(l[3:])
            idx := strings.index(l, "=>")
            subkey := strings.trim_space(l if idx == -1 else l[:idx])
            value := "" if idx == -1 else strings.trim_space(l[idx+2:])
            set_sub_value(&manifest, current_section, last_key, subkey, value, allocator)
        }
    }
    return manifest
}

set_value :: proc(m: ^Manifest, section, key, value: string, allocator: mem.Allocator) {
    switch section {
        case "metadata":
            switch key {
                case "name": m.name = strings.clone(value, allocator)
                case "version": m.version = strings.clone(value, allocator)
                case "authors": m.authors = strings.clone(value, allocator)
                case "license": m.license = strings.clone(value, allocator)
            }
                case "description":
                    switch key {
                        case "summary": m.summary = strings.clone(value, allocator)
                        case "long": m.long = strings.clone(value, allocator)
                    }
                        case "specs":
                            if key != "dependencies" {
                                m.system_specs[strings.clone(key, allocator)] = strings.clone(value, allocator)
                            }
                        case "sandbox":
                            switch key {
                                case "network": m.sandbox.network = value == "true"
                                case "gui": m.sandbox.gui = value == "true"
                                case "dev": m.sandbox.dev = value == "true"
                            }
    }
}

set_sub_value :: proc(m: ^Manifest, section, last_key, subkey, value: string, allocator: mem.Allocator) {
    switch section {
        case "metadata":
            if last_key == "bins" && value == "" {
                append(&m.bins, strings.clone(subkey, allocator))
            }
        case "specs":
            if last_key == "dependencies" {
                m.deps[strings.clone(subkey, allocator)] = strings.clone(value, allocator)
            }
        case "sandbox":
            if last_key == "filesystem" && value == "" {
                append(&m.sandbox.filesystem, strings.clone(subkey, allocator))
            }
        case "install":
            if last_key == "commands" && value == "" {
                append(&m.install_commands, strings.clone(subkey, allocator))
            }
    }
}

install :: proc(allocator: mem.Allocator, package_name: string, version: string, path: string, checksum: string) {
    tmp_path := fmt.aprintf("{}.tmp", path)
    defer delete(tmp_path)
    merr := os.make_directory(tmp_path)
    if merr != nil {
        ge, ok := merr.(os.General_Error)
        if !(ok && ge == .Exist) {
            panic("Failed to create tmp directory")
        }
    }
    // Move contents/* to tmp_path
    contents_path := fmt.aprintf("{}/contents", tmp_path)
    defer delete(contents_path)
    if os.exists(contents_path) {
        dir, oerr := os.open(contents_path, os.O_RDONLY)
        if oerr != nil { panic("Open contents failed") }
        defer os.close(dir)
        entries, rerr := os.read_dir(dir, -1)
        if rerr != nil { panic("Read dir failed") }
        defer {
            for entry in entries {
                delete(entry.name, allocator)
            }
            delete(entries, allocator)
        }
        for entry in entries {
            old_p := fmt.aprintf("{}/{}", contents_path, entry.name)
            new_p := fmt.aprintf("{}/{}", tmp_path, entry.name)
            defer {
                delete(old_p, allocator)
                delete(new_p, allocator)
            }
            rerr := os.rename(old_p, new_p)
            if rerr != nil { panic("Move failed") }
        }
        os.remove_directory(contents_path)
    }
    manifest := load_info(allocator, tmp_path)
    defer deinit_manifest(&manifest, allocator)
    if len(manifest.deps) > 0 {
        for dep, req in manifest.deps {
            // TODO: Deps handled in CLI
            fmt.eprintf("Dependency: {} {}\n", dep, req)
        }
    }
    setup_sandbox(allocator, tmp_path, &manifest, true)
    verify(allocator, tmp_path, checksum)
    rerr := os.rename(tmp_path, path)
    if rerr != nil {
        panic("Rename failed")
    }
    update_state(allocator, package_name, version, checksum)
    payload := struct { success: bool, package_name: string }{true, package_name}
    output, merr2 := json.marshal(payload)
    if merr2 != nil {
        panic("JSON marshal failed")
    }
    defer delete(output)
    os.write(os.stdout, output)
}

remove :: proc(allocator: mem.Allocator, package_name: string, version: string, path: string) {
    manifest := load_info(allocator, path)
    defer deinit_manifest(&manifest, allocator)
    if len(manifest.bins) > 0 {
        for bin in manifest.bins {
            bin_path := fmt.aprintf("/usr/bin/{}", bin)
            defer delete(bin_path, allocator)
            _ = os.remove(bin_path)
        }
    }
    if err := delete_tree(path); err != nil {
        panic("Delete tree failed")
    }
    state := load_state(allocator)
    defer deinit_state(&state, allocator)
    if vers, found := state.packages[package_name]; found {
        vers_copy := vers
        if checksum, ok := vers_copy[version]; ok {
            delete(checksum, allocator)
            delete_key(&vers_copy, version)
            if len(vers_copy) == 0 {
                delete(vers_copy)
                delete_key(&state.packages, package_name)
            } else {
                state.packages[package_name] = vers_copy
            }
        }
    }
    save_state(&state)
    payload := struct { success: bool, package_name: string }{true, package_name}
    output, merr := json.marshal(payload)
    if merr != nil {
        panic("JSON marshal failed")
    }
    defer delete(output)
    os.write(os.stdout, output)
}

verify :: proc(allocator: mem.Allocator, path: string, checksum: string) {
    info_path := fmt.aprintf("{}/info.hk", path)
    defer delete(info_path, allocator)
    data, ok := os.read_entire_file(info_path)
    if !ok {
        panic("Failed to read info.hk for verify")
    }
    defer delete(data, allocator)
    ctx: sha2.Context_256
    sha2.init_256(&ctx)
    sha2.update(&ctx, data)
    hash: [sha2.DIGEST_SIZE_256]u8
    sha2.final(&ctx, hash[:])
    computed_builder: strings.Builder
    strings.builder_init(&computed_builder, allocator)
    defer strings.builder_destroy(&computed_builder)
    for b in hash {
        fmt.sbprintf(&computed_builder, "{:02x}", b)
    }
    computed := strings.to_string(computed_builder)
    if computed != checksum {
        output_error(allocator, .VerificationFailed, "Checksum mismatch")
    }
}

setup_sandbox :: proc(allocator: mem.Allocator, path: string, manifest: ^Manifest, is_install: bool, bin: string = "", extra_args: []string = nil) {
    args: [dynamic]string
    defer {
        for arg in args {
            delete(arg, allocator)
        }
        delete(args)
    }
    append(&args, strings.clone("bwrap", allocator))
    append(&args, strings.clone("--ro-bind", allocator), strings.clone("/usr", allocator), strings.clone("/usr", allocator))
    append(&args, strings.clone("--ro-bind", allocator), strings.clone("/lib", allocator), strings.clone("/lib", allocator))
    append(&args, strings.clone("--ro-bind", allocator), strings.clone("/lib64", allocator), strings.clone("/lib64", allocator))
    append(&args, strings.clone("--ro-bind", allocator), strings.clone("/bin", allocator), strings.clone("/bin", allocator))
    append(&args, strings.clone("--ro-bind", allocator), strings.clone("/etc", allocator), strings.clone("/etc", allocator))
    append(&args, strings.clone("--bind", allocator), strings.clone(path, allocator), strings.clone("/app", allocator))
    append(&args, strings.clone("--chdir", allocator), strings.clone("/app", allocator))
    append(&args, strings.clone("--unshare-all", allocator))
    if manifest.sandbox.network {
        append(&args, strings.clone("--share-net", allocator))
    } else {
        append(&args, strings.clone("--unshare-net", allocator))
    }
    if manifest.sandbox.gui {
        append(&args, strings.clone("--ro-bind", allocator), strings.clone("/tmp/.X11-unix", allocator), strings.clone("/tmp/.X11-unix", allocator))
        append(&args, strings.clone("--share-ipc", allocator))
        display := os.get_env("DISPLAY", allocator)
        defer delete(display, allocator)
        append(&args, strings.clone("--set-var", allocator), strings.clone("DISPLAY", allocator), display)
    }
    if manifest.sandbox.dev {
        append(&args, strings.clone("--dev-bind", allocator), strings.clone("/dev", allocator), strings.clone("/dev", allocator))
    }
    if len(manifest.sandbox.filesystem) > 0 {
        for fs_path in manifest.sandbox.filesystem {
            append(&args, strings.clone("--bind", allocator), strings.clone(fs_path, allocator), strings.clone(fs_path, allocator))
        }
    }
    if is_install {
        install_cmd := "echo 'Isolated install complete'"
        if len(manifest.install_commands) > 0 {
            sb: strings.Builder
            strings.builder_init(&sb, allocator)
            defer strings.builder_destroy(&sb)
            for cmd, i in manifest.install_commands {
                if i > 0 { fmt.sbprint(&sb, " && ") }
                fmt.sbprint(&sb, cmd)
            }
            install_cmd = strings.to_string(sb)
        }
        append(&args, strings.clone("--", allocator), strings.clone("sh", allocator), strings.clone("-c", allocator), strings.clone(install_cmd, allocator))
    } else {
        bin_path := fmt.aprintf("/app/{}", bin)
        defer delete(bin_path, allocator)
        append(&args, strings.clone("--", allocator), bin_path)
        for arg in extra_args {
            append(&args, strings.clone(arg, allocator))
        }
    }
    code := run_command(args[:])
    if code != 0 {
        if is_install {
            output_error(allocator, .InstallFailed, "Sandbox failed")
        } else {
            os.exit(int(code))
        }
    }
}

run :: proc(allocator: mem.Allocator, args: []string) {
    package_name := args[0]
    bin := args[1]
    extra_args := args[2:]
    path := fmt.aprintf("{}{}/current", STORE_PATH, package_name)
    defer delete(path, allocator)
    manifest := load_info(allocator, path)
    defer deinit_manifest(&manifest, allocator)
    setup_sandbox(allocator, path, &manifest, false, bin, extra_args)
}

run_command :: proc(argv: []string) -> i32 {
    pid, ferr := linux.fork()
    if ferr != .NONE {
        panic("Fork failed")
    }
    if pid == 0 {
        c_argv := make([]cstring, len(argv) + 1, context.temp_allocator)
        for arg, i in argv {
            c_argv[i] = strings.clone_to_cstring(arg, context.temp_allocator)
        }
        c_argv[len(argv)] = nil
        path := strings.clone_to_cstring(argv[0], context.temp_allocator)
        err := linux.execve(path, raw_data(c_argv), nil)
        linux.exit(127)
    }
    status: u32
    _, werr := linux.waitpid(pid, &status, {}, nil)
    if werr != .NONE {
        return -1
    }
    if linux.WIFEXITED(status) {
        return i32(linux.WEXITSTATUS(status))
    }
    return -1
}

delete_tree :: proc(path: string) -> os.Error {
    dir, open_err := os.open(path, os.O_RDONLY)
    if open_err != nil {
        return open_err
    }
    defer os.close(dir)
    entries, read_err := os.read_dir(dir, -1)
    if read_err != nil {
        return read_err
    }
    defer {
        for entry in entries {
            delete(entry.name)
        }
        delete(entries)
    }
    for entry in entries {
        full_path := fmt.tprintf("%s/%s", path, entry.name)
        defer delete(full_path)
        if entry.is_dir {
            if del_err := delete_tree(full_path); del_err != nil {
                return del_err
            }
        } else {
            if rem_err := os.remove(full_path); rem_err != nil {
                return rem_err
            }
        }
    }
    return os.remove_directory(path)
}

STATE_PATH :: "/var/lib/hpm/state.json"

load_state :: proc(allocator: mem.Allocator) -> State {
    data, ok := os.read_entire_file(STATE_PATH)
    if !ok {
        return State{packages = {}}
    }
    defer delete(data)
    state: State
    err := json.unmarshal(data, &state, allocator = allocator)
    if err != nil {
        panic("Failed to parse state")
    }
    return state
}

save_state :: proc(state: ^State) {
    file, open_err := os.open(STATE_PATH, os.O_CREATE | os.O_WRONLY | os.O_TRUNC)
    if open_err != nil {
        panic("Failed to open state file")
    }
    defer os.close(file)
    data, marshal_err := json.marshal(state^)
    if marshal_err != nil {
        panic("JSON marshal failed")
    }
    defer delete(data)
    _, write_err := os.write(file, data)
    if write_err != nil {
        panic("Failed to write state file")
    }
}

update_state :: proc(allocator: mem.Allocator, package_name: string, version: string, checksum: string) {
    state := load_state(allocator)
    defer deinit_state(&state, allocator)
    pkg_key := strings.clone(package_name, allocator)
    ver_key := strings.clone(version, allocator)
    chk := strings.clone(checksum, allocator)
    if _, ok := state.packages[pkg_key]; !ok {
        state.packages[pkg_key] = {}
    }
    inner := state.packages[pkg_key]
    inner[ver_key] = chk
    state.packages[pkg_key] = inner
    save_state(&state)
}
