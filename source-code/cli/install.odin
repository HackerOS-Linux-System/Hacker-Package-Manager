package hpm
import "core:fmt"
import "core:os"
import "core:mem"
import "core:strings"
import "core:path/filepath"
import "core:sys/linux"
import "core:time"
import "core:encoding/json"
Manifest :: struct {
    bins: [dynamic]string,
    author: string,
    license: string,
    description: string,
    deps: [dynamic]string, // For info.hk deps if any
}
load_manifest :: proc(allocator: mem.Allocator, path: string) -> (Manifest, Error) {
    info_path := filepath.join({path, "info.hk"})
    defer delete(info_path)
    data, ok := os.read_entire_file(info_path, allocator)
    if !ok {
        return {}, .BackendFailed
    }
    defer delete(data)
    m: struct {
        bins: []string,
        author: string,
        license: string,
        description: string,
        deps: []string,
    }
    err := json.unmarshal(data, &m, allocator = allocator)
    if err != nil {
        return {}, .BackendFailed
    }
    bins := make([dynamic]string, len(m.bins), allocator)
    for b, i in m.bins {
        bins[i] = strings.clone(b, allocator)
    }
    deps := make([dynamic]string, len(m.deps), allocator)
    for d, i in m.deps {
        deps[i] = strings.clone(d, allocator)
    }
    manifest := Manifest {
        bins = bins,
        author = m.author,
        license = m.license,
        description = m.description,
        deps = deps,
    }
    return manifest, .None
}
deinit_manifest :: proc(m: ^Manifest, allocator: mem.Allocator) {
    for str in m.bins {
        delete(str, allocator)
    }
    delete(m.bins)
    for str in m.deps {
        delete(str, allocator)
    }
    delete(m.deps)
    delete(m.author, allocator)
    delete(m.license, allocator)
    delete(m.description, allocator)
}
refresh :: proc(allocator: mem.Allocator) -> Error {
    log_to_file("INFO", "Refreshing package index")
    temp_path := "/usr/lib/HackerOS/hpm/repo.json"
    err := download_file(allocator, REPO_JSON_URL, temp_path)
    if err != .None {
        log_to_file("ERROR", "Download failed for repo.json")
        return err
    }
    if os.rename(temp_path, "/usr/lib/HackerOS/hpm/repo.json") != os.ERROR_NONE {
        return .BackendFailed
    }
    fmt.printf("%s✔ Package index refreshed.%s\n", COLOR_GREEN, COLOR_RESET)
    return .None
}
install :: proc(allocator: mem.Allocator, args: []string) -> Error {
    lock_err := acquire_lock()
    if lock_err != .None {
        return lock_err
    }
    defer release_lock()
    log_to_file("INFO", fmt.tprintf("Installing %s", strings.join(args, " ")))
    repo, repo_err := load_repo(allocator)
    if repo_err != .None {
        return repo_err
    }
    defer deinit_repo(&repo, allocator)
    state, state_err := load_state(allocator)
    if state_err != .None {
        return state_err
    }
    defer delete_state(&state, allocator)
    installed, inst_err := get_installed(allocator, &state)
    if inst_err != .None {
        return inst_err
    }
    defer {
        for k, v in installed {
            delete(k, allocator)
            delete(v, allocator)
        }
        delete(installed)
    }
    summary_deps: [dynamic]string
    summary_bins: [dynamic]string
    defer {
        for str in summary_deps {
            delete(str)
        }
        delete(summary_deps)
        for str in summary_bins {
            delete(str)
        }
        delete(summary_bins)
    }
    for spec in args {
        parts := strings.split(spec, "@")
        defer delete(parts)
        pkg_name := parts[0]
        requested_ver := len(parts) > 1 ? parts[1] : ""
        req := requested_ver != "" ? fmt.tprintf("=%s", requested_ver) : ""
        chosen: map[string]string
        order: [dynamic]struct {pkg: string, ver: string}
        defer {
            for k, v in chosen {
                delete(k, allocator)
                delete(v, allocator)
            }
            delete(chosen)
            for item in order {
                delete(item.pkg)
                delete(item.ver)
            }
            delete(order)
        }
        res_err := resolve_deps_iterative(allocator, &repo, pkg_name, req, &chosen, &order)
        if res_err != .None {
            return res_err
        }
        for item in order {
            p := item.pkg
            v := item.ver
            if inst_ver, ok := installed[p]; ok && satisfies(inst_ver, fmt.tprintf("=%s", v)) {
                fmt.printf("%s➤ %s@%s already installed.%s\n", COLOR_YELLOW, p, v, COLOR_RESET)
                continue
            }
            single_err := install_single(allocator, p, v, &repo, &state)
            if single_err != .None {
                return single_err
            }
            append(&summary_deps, fmt.tprintf("%s%s@%s%s", COLOR_CYAN, p, v, COLOR_RESET))
            // Load manifest for bins
            pkg_path := fmt.tprintf("%s%s/%s", STORE_PATH, p, v)
            defer delete(pkg_path)
            manifest, man_err := load_manifest(allocator, pkg_path)
            if man_err == .None {
                for bin in manifest.bins {
                    append(&summary_bins, fmt.tprintf("%s%s%s", COLOR_MAGENTA, bin, COLOR_RESET))
                }
                deinit_manifest(&manifest, allocator)
            }
        }
    }
    // Podsumowanie
    if len(summary_deps) > 0 {
        fmt.printf("%sInstalled dependencies:%s\n", COLOR_BLUE, COLOR_RESET)
        for d in summary_deps {
            fmt.printf(" - %s\n", d)
        }
    }
    if len(summary_bins) > 0 {
        fmt.printf("%sAdded binaries to /usr/bin/:%s\n", COLOR_BLUE, COLOR_RESET)
        for b in summary_bins {
            fmt.printf(" - %s\n", b)
        }
    }
    err_save := save_state(&state, allocator)
    if err_save != .None {
        return err_save
    }
    return .None
}
install_single :: proc(allocator: mem.Allocator, package_name: string, version: string, repo: ^Repo, state: ^StatePackages) -> Error {
    log_to_file("INFO", fmt.tprintf("Installing single %s@%s", package_name, version))
    pkg, ok := repo^[package_name]
    if !ok {
        return .PackageNotFound
    }
    ver_obj: struct {version: string, url: string, sha256: string, deps: map[string]string}
    found := false
    for v in pkg.versions {
        if v.version == version {
            ver_obj = v
            found = true
            break
        }
    }
    if !found {
        log_to_file("ERROR", fmt.tprintf("Version %s not found for %s", version, package_name))
        return .VersionNotFound
    }
    pkg_url := ver_obj.url
    expected_sha := ver_obj.sha256
    pkg_path := fmt.tprintf("%s%s/%s", STORE_PATH, package_name, version)
    defer delete(pkg_path)
    current_link := fmt.tprintf("%s%s/current", STORE_PATH, package_name)
    defer delete(current_link)
    if os.exists(pkg_path) {
        fmt.printf("%s✔ Already installed %s%s@%s%s%s\n", COLOR_GREEN, COLOR_CYAN, package_name, version, COLOR_RESET, COLOR_RESET)
        return .None
    }
    cache_archive := fmt.tprintf("%s%s-%s.hpm", CACHE_PATH, package_name, version)
    defer delete(cache_archive)
    os.make_directory(CACHE_PATH, 0o755)
    if os.exists(cache_archive) {
        fmt.printf("%sUsing cached archive for %s@%s%s\n", COLOR_YELLOW, package_name, version, COLOR_RESET)
    } else {
        down_err := download_file(allocator, pkg_url, cache_archive)
        if down_err != .None {
            log_to_file("ERROR", "Download failed")
            return down_err
        }
    }
    if expected_sha != "" {
        computed_sha, sha_err := compute_sha256_stream(allocator, cache_archive)
        defer delete(computed_sha)
        if sha_err != .None || computed_sha != expected_sha {
            log_to_file("ERROR", "SHA256 mismatch")
            os.remove(cache_archive)
            return .ChecksumMismatch
        }
    }
    temp_extract := fmt.tprintf("%s.tmp", pkg_path)
    defer delete(temp_extract)
    os.remove_directory(temp_extract)
    os.make_directory(temp_extract, 0o755)
    done, t := start_spinner()
    defer stop_spinner(done, t)
    unpack_args := []string{"tar", "-I", "zstd", "-xf", cache_archive, "-C", temp_extract}
    code, run_err := run_command(unpack_args[:])
    if code != 0 || run_err != .None {
        log_to_file("ERROR", "Unpack failed")
        return .UnpackFailed
    }
    checksum := expected_sha != "" ? expected_sha : "none"
    backend_args := []string{BACKEND_PATH, "install", package_name, version, temp_extract, checksum}
    code, run_err = run_command(backend_args[:])
    if code != 0 || run_err != .None {
        log_to_file("ERROR", "Backend install failed")
        return .BackendFailed
    }
    if os.rename(temp_extract, pkg_path) != os.ERROR_NONE {
        return .BackendFailed
    }
    os.make_directory(filepath.dir(current_link), 0o755)
    if os.exists(current_link) {
        os.remove(current_link)
    }
    symlink_err := linux.symlink(strings.clone_to_cstring(version, context.temp_allocator), strings.clone_to_cstring(current_link, context.temp_allocator))
    if symlink_err != .NONE {
        log_to_file("ERROR", "Failed to create symlink")
        return .SymlinkFailed
    }
    manifest, man_err := load_manifest(allocator, pkg_path)
    if man_err != .None {
        return man_err
    }
    defer deinit_manifest(&manifest, allocator)
    for bin in manifest.bins {
        wrapper_path := fmt.tprintf("/usr/bin/%s", bin)
        defer delete(wrapper_path)
        wrapper_content := fmt.tprintf("#!/bin/sh\nexec %s run %s %s \"$@\"\n", BACKEND_PATH, package_name, bin)
        defer delete(wrapper_content)
        os.write_entire_file(wrapper_path, transmute([]u8)wrapper_content)
        if linux.chmod(strings.clone_to_cstring(wrapper_path, context.temp_allocator), {.IRUSR, .IWUSR, .IXUSR, .IRGRP, .IXGRP, .IROTH, .IXOTH}) != .NONE {
            log_to_file("ERROR", "Failed to chmod wrapper")
            return .ChmodFailed
        }
    }
    // Update state with date
    if _, ok := state^[package_name]; !ok {
        state^[package_name] = make(map[string]VersionInfo, allocator)
    }
    vers := state^[package_name]
    vers[version] = VersionInfo{checksum = checksum, date = time.now(), pinned = false}
    fmt.printf("%s✔ Installed %s%s@%s%s%s\n", COLOR_GREEN, COLOR_CYAN, package_name, version, COLOR_RESET, COLOR_RESET)
    return .None
}
verify :: proc(allocator: mem.Allocator, pkg_name: string) -> Error {
    state, state_err := load_state(allocator)
    if state_err != .None {
        return state_err
    }
    defer delete_state(&state, allocator)
    vers, ok := state[pkg_name]
    if !ok || len(vers) == 0 {
        return .PackageNotFound
    }
    current_link := fmt.tprintf("%s%s/current", STORE_PATH, pkg_name)
    defer delete(current_link)
    target, rok := readlink(current_link, allocator)
    if !rok {
        return .VersionNotFound
    }
    defer delete(target)
    ver := filepath.base(target)
    info, vok := vers[ver]
    if !vok {
        return .VersionNotFound
    }
    pkg_path := fmt.tprintf("%s%s/%s", STORE_PATH, pkg_name, ver)
    defer delete(pkg_path)
    backend_args := []string{BACKEND_PATH, "verify", pkg_name, ver, pkg_path, info.checksum}
    code, run_err := run_command(backend_args[:])
    if code != 0 || run_err != .None {
        fmt.printf("%sVerification failed for %s@%s.%s\n", COLOR_RED, pkg_name, ver, COLOR_RESET)
        return .VerifyFailed
    }
    fmt.printf("%s✔ Verification successful for %s@%s.%s\n", COLOR_GREEN, pkg_name, ver, COLOR_RESET)
    return .None
}
