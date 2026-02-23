package hpm

import "core:fmt"
import "core:os"
import "core:mem"
import "core:strings"
import "core:path/filepath"

remove :: proc(allocator: mem.Allocator, pkg_spec: string, non_interactive: bool = false) -> Error {
    lock_err := acquire_lock()
    if lock_err != .None {
        return lock_err
    }
    defer release_lock()
    log_to_file("INFO", fmt.tprintf("Removing %s", pkg_spec))
    state, state_err := load_state(allocator)
    if state_err != .None {
        return state_err
    }
    defer delete_state(&state, allocator)
    parts := strings.split(pkg_spec, "@")
    defer delete(parts, allocator)
    pkg_name := parts[0]
    version := len(parts) > 1 ? parts[1] : ""
    if _, ok := state[pkg_name]; !ok {
        fmt.printf("%sPackage %s%s%s not installed.%s\n", COLOR_RED, COLOR_CYAN, pkg_name, COLOR_RESET, COLOR_RESET)
        return .PackageNotFound
    }
    vers_map := state[pkg_name]
    confirm_all := version == ""
    if !non_interactive {
        if confirm_all {
            fmt.printf("%sAre you sure you want to remove all versions of %s%s%s? [y/N] %s", COLOR_YELLOW, COLOR_CYAN, pkg_name, COLOR_RESET, COLOR_RESET)
        } else {
            fmt.printf("%sAre you sure you want to remove %s%s@%s%s? [y/N] %s", COLOR_YELLOW, COLOR_CYAN, pkg_name, version, COLOR_RESET, COLOR_RESET)
        }
        input: [1024]u8
        n, _ := os.read(os.stdin, input[:])
        resp := strings.trim_space(string(input[:n]))
        if !strings.equal_fold(resp, "y") {
            fmt.printf("%sRemoval cancelled.%s\n", COLOR_YELLOW, COLOR_RESET)
            return .None
        }
    }
    current_link := fmt.tprintf("%s%s/current", STORE_PATH, pkg_name)
    defer delete(current_link, allocator)
    if version != "" {
        if _, ok := vers_map[version]; !ok {
            fmt.printf("%sVersion %s not installed.%s\n", COLOR_RED, version, COLOR_RESET)
            return .VersionNotFound
        }
        installed_path := fmt.tprintf("%s%s/%s", STORE_PATH, pkg_name, version)
        defer delete(installed_path, allocator)
        backend_args := []string{BACKEND_PATH, "remove", pkg_name, version, installed_path}
        _, run_err := run_command(backend_args[:])
        if run_err != .None {
            return run_err
        }
        os.remove_directory(installed_path)
        target, ok := readlink(current_link, allocator)
        if ok {
            if filepath.base(target) == version {
                os.remove(current_link)
            }
            delete(target, allocator)
        }
        delete_key(&vers_map, version)
        if len(vers_map) == 0 {
            delete_key(&state, pkg_name)
            os.remove_directory(fmt.tprintf("%s%s", STORE_PATH, pkg_name))
        }
    } else {
        vers_keys: [dynamic]string
        defer delete(vers_keys)
        for ver_key in vers_map {
            append(&vers_keys, strings.clone(ver_key, allocator))
        }
        for ver in vers_keys {
            installed_path := fmt.tprintf("%s%s/%s", STORE_PATH, pkg_name, ver)
            defer delete(installed_path, allocator)
            backend_args := []string{BACKEND_PATH, "remove", pkg_name, ver, installed_path}
            _, run_err := run_command(backend_args[:])
            if run_err != .None {
                return run_err
            }
            os.remove_directory(installed_path)
            delete_key(&vers_map, ver)
        }
        os.remove_directory(fmt.tprintf("%s%s", STORE_PATH, pkg_name))
        delete_key(&state, pkg_name)
    }
    err_save := save_state(&state, allocator)
    if err_save != .None {
        return err_save
    }
    fmt.printf("%s✔ %s removed.%s\n", COLOR_GREEN, pkg_spec, COLOR_RESET)
    return .None
}
