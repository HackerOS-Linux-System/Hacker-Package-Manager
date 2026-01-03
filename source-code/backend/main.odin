package main

import "core:fmt"
import "core:os"
import "core:mem"
import "core:strings"
import "core:encoding/json"
import "core:crypto/sha2"
import "core:sys/linux"

Manifest :: struct {
    name: string,
    version: string,
    deps: []string,
    bins: []string,
    sandbox: struct {
        network: bool,
        filesystem: []string,
    },
}

deinit_manifest :: proc(self: ^Manifest, allocator: mem.Allocator) {
    delete(self.name, allocator)
    delete(self.version, allocator)
    if self.deps != nil {
        for dep in self.deps {
            delete(dep, allocator)
        }
        delete(self.deps, allocator)
    }
    if self.bins != nil {
        for bin in self.bins {
            delete(bin, allocator)
        }
        delete(self.bins, allocator)
    }
    if self.sandbox.filesystem != nil {
        for path in self.sandbox.filesystem {
            delete(path, allocator)
        }
        delete(self.sandbox.filesystem, allocator)
    }
}

PackageInfo :: struct {
    version: string,
    checksum: string,
}

State :: struct {
    packages: map[string]PackageInfo,
}

deinit_state :: proc(self: ^State, allocator: mem.Allocator) {
    for key, value in self.packages {
        delete(value.version, allocator)
        delete(value.checksum, allocator)
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

main :: proc() {
    arena: mem.Arena
    backing := make([]u8, 8 * mem.Megabyte)
    mem.arena_init(&arena, backing)
    defer delete(backing)
    allocator := mem.arena_allocator(&arena)
    context.allocator = allocator
    args := os.args[1:]
    if len(args) < 3 {
        output_error(allocator, .InvalidArgs, "Usage: backend [install|remove|verify] <package> <path> [checksum]")
        return
    }
    command := args[0]
    package_name := args[1]
    path := args[2]
    checksum: Maybe(string) = nil
    if len(args) > 3 {
        checksum = args[3]
    }
    switch command {
        case "install":
            install(allocator, package_name, path, checksum)
        case "remove":
            remove(allocator, package_name, path)
        case "verify":
            if checksum == nil {
                output_error(allocator, .InvalidArgs, "Checksum required for verify")
                return
            }
            verify(allocator, path, checksum.?)
            payload := struct { success: bool }{true}
            output, merr := json.marshal(payload)
            if merr != nil {
                panic("JSON marshal failed")
            }
            defer delete(output)
            os.write(os.stdout, output)
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

load_manifest :: proc(allocator: mem.Allocator, path: string) -> Manifest {
    manifest_path := fmt.aprintf("{}/manifest.json", path)
    defer delete(manifest_path)
    data, ok := os.read_entire_file(manifest_path)
    if !ok {
        panic("Failed to read manifest")
    }
    defer delete(data)
    manifest: Manifest
    err := json.unmarshal(data, &manifest, allocator = allocator)
    if err != nil {
        panic("Failed to parse manifest")
    }
    return manifest
}

install :: proc(allocator: mem.Allocator, package_name: string, path: string, checksum: Maybe(string)) {
    tmp_path := fmt.aprintf("{}.tmp", path)
    defer delete(tmp_path)
    merr := os.make_directory(tmp_path)
    if merr != nil {
        ge, ok := merr.(os.General_Error)
        if !(ok && ge == .Exist) {
            panic("Failed to create tmp directory")
        }
    }
    manifest := load_manifest(allocator, tmp_path)
    defer deinit_manifest(&manifest, allocator)
    if manifest.deps != nil && len(manifest.deps) > 0 {
        for dep in manifest.deps {
            // TODO: Full DAG, cycle detection
            fmt.eprintf("Installing dep: {}\n", dep)
        }
    }
    setup_sandbox(allocator, package_name, tmp_path, &manifest)
    if chk, ok := checksum.?; ok {
        verify(allocator, tmp_path, chk)
    }
    rerr := os.rename(tmp_path, path)
    if rerr != nil {
        panic("Rename failed")
    }
    checksum_str := "none"
    if c, ok := checksum.?; ok {
        checksum_str = c
    }
    update_state(allocator, package_name, manifest.version, checksum_str)
    payload := struct { success: bool, package_name: string }{true, package_name}
    output, merr2 := json.marshal(payload)
    if merr2 != nil {
        panic("JSON marshal failed")
    }
    defer delete(output)
    os.write(os.stdout, output)
}

remove :: proc(allocator: mem.Allocator, package_name: string, path: string) {
    manifest := load_manifest(allocator, path)
    defer deinit_manifest(&manifest, allocator)
    if manifest.bins != nil && len(manifest.bins) > 0 {
        for bin in manifest.bins {
            bin_path := fmt.aprintf("/usr/bin/{}", bin)
            defer delete(bin_path)
            _ = os.remove(bin_path)
        }
    }
    if err := delete_tree(path); err != nil {
        panic("Delete tree failed")
    }
    state := load_state(allocator)
    defer deinit_state(&state, allocator)
    if pi, found := state.packages[package_name]; found {
        delete(pi.version, allocator)
        delete(pi.checksum, allocator)
        delete_key(&state.packages, package_name)
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
    manifest_path := fmt.aprintf("{}/manifest.json", path)
    defer delete(manifest_path)
    data, ok := os.read_entire_file(manifest_path)
    if !ok {
        panic("Failed to read manifest for verify")
    }
    defer delete(data)
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

setup_sandbox :: proc(allocator: mem.Allocator, package_name: string, path: string, manifest: ^Manifest) {
    _ = package_name // unused
    args: [dynamic]string
    append(&args, "bwrap")
    append(&args, "--ro-bind", "/usr", "/usr")
    append(&args, "--ro-bind", "/lib", "/lib")
    append(&args, "--ro-bind", "/lib64", "/lib64")
    append(&args, "--ro-bind", "/bin", "/bin")
    append(&args, "--ro-bind", "/etc", "/etc")
    append(&args, "--bind", path, "/app")
    append(&args, "--chdir", "/app")
    append(&args, "--unshare-all")
    if !manifest.sandbox.network {
        append(&args, "--unshare-net")
    } else {
        append(&args, "--share-net")
    }
    if manifest.sandbox.filesystem != nil && len(manifest.sandbox.filesystem) > 0 {
        for fs_path in manifest.sandbox.filesystem {
            append(&args, "--bind", fs_path, fs_path)
        }
    }
    append(&args, "--", "sh", "-c", "echo Isolated install complete") // Replace with actual
    code := run_command(args[:])
    delete(args)
    if code != 0 {
        output_error(allocator, .InstallFailed, "Sandbox failed")
    }
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
    state.packages[strings.clone(package_name, allocator)] = PackageInfo{
        version = strings.clone(version, allocator),
        checksum = strings.clone(checksum, allocator),
    }
    save_state(&state)
}
