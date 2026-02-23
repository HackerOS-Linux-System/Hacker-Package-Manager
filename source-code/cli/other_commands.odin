package hpm

import "core:fmt"
import "core:os"
import "core:mem"
import "core:strings"
import "core:path/filepath"
import "core:sys/linux"
import "core:time"
import "core:encoding/json"
import "core:sort"

switch_version :: proc(allocator: mem.Allocator, pkg_name: string, version: string) -> Error {
    lock_err := acquire_lock()
    if lock_err != .None {
        return lock_err
    }
    defer release_lock()
    log_to_file("INFO", fmt.tprintf("Switching %s to %s", pkg_name, version))
    state, state_err := load_state(allocator)
    if state_err != .None {
        return state_err
    }
    defer delete_state(&state, allocator)
    if _, ok := state[pkg_name]; !ok {
        return .PackageNotFound
    }
    vers_map := state[pkg_name]
    if _, ok := vers_map[version]; !ok {
        return .VersionNotFound
    }
    current_link := fmt.tprintf("%s%s/current", STORE_PATH, pkg_name)
    defer delete(current_link)
    if os.exists(current_link) {
        os.remove(current_link)
    }
    symlink_err := linux.symlink(strings.clone_to_cstring(version, context.temp_allocator), strings.clone_to_cstring(current_link, context.temp_allocator))
    if symlink_err != .NONE {
        log_to_file("ERROR", "Failed to switch version")
        return .SwitchFailed
    }
    save_state(&state, allocator)
    fmt.printf("%s✔ Switched %s%s%s to %s%s%s.%s\n", COLOR_GREEN, COLOR_CYAN, pkg_name, COLOR_RESET, COLOR_CYAN, version, COLOR_RESET, COLOR_RESET)
    return .None
}

upgrade :: proc(allocator: mem.Allocator) -> Error {
    log_to_file("INFO", "Upgrading HPM")
    temp_version_file := "/tmp/hpm-version.hacker"
    down_err := download_file(allocator, VERSION_URL, temp_version_file)
    if down_err != .None {
        return down_err
    }
    remote_raw, ok := os.read_entire_file(temp_version_file, allocator)
    if !ok {
        return .DownloadFailed
    }
    defer delete(remote_raw)
    replaced, was_allocation := strings.replace_all(string(remote_raw), "[]", "", allocator)
    defer if was_allocation { delete(replaced) }
    remote_version := strings.trim_space(replaced)
    local_data, lok := os.read_entire_file(LOCAL_VERSION_FILE, allocator)
    local_version := "0.0"
    if lok {
        defer delete(local_data)
        lstate: struct {version: string}
        json.unmarshal(local_data, &lstate, allocator = allocator)
        local_version = lstate.version
    }
    if compare_versions(remote_version, local_version) > 0 {
        hpm_url := fmt.tprintf("%s%s/hpm", RELEASES_BASE, remote_version)
        defer delete(hpm_url)
        down_err = download_file(allocator, hpm_url, "/usr/bin/hpm")
        if down_err != .None {
            return down_err
        }
        if linux.chmod(strings.clone_to_cstring("/usr/bin/hpm", context.temp_allocator), {.IRUSR, .IWUSR, .IXUSR, .IRGRP, .IXGRP, .IROTH, .IXOTH}) != .NONE {
            return .ChmodFailed
        }
        backend_url := fmt.tprintf("%s%s/backend", RELEASES_BASE, remote_version)
        defer delete(backend_url)
        down_err = download_file(allocator, backend_url, BACKEND_PATH)
        if down_err != .None {
            return down_err
        }
        if linux.chmod(strings.clone_to_cstring(BACKEND_PATH, context.temp_allocator), {.IRUSR, .IWUSR, .IXUSR, .IRGRP, .IXGRP, .IROTH, .IXOTH}) != .NONE {
            return .ChmodFailed
        }
        new_version := struct {version: string}{remote_version}
        data, merr := json.marshal(new_version, allocator = allocator)
        if merr != nil {
            return .BackendFailed
        }
        defer delete(data)
        os.write_entire_file(LOCAL_VERSION_FILE, data)
        fmt.printf("%s✔ Upgrade complete to %s.%s\n", COLOR_GREEN, remote_version, COLOR_RESET)
    } else {
        fmt.printf("%s✔ HPM is up to date.%s\n", COLOR_GREEN, COLOR_RESET)
    }
    return .None
}

run_tool :: proc(allocator: mem.Allocator, args: []string) -> int {
    if len(args) < 2 {
        print_error(.InvalidArgs)
        return 1
    }
    package_spec := args[0]
    bin := args[1]
    extra_args := args[2:]
    parts := strings.split(package_spec, "@")
    defer delete(parts)
    pkg_name := parts[0]
    version := len(parts) > 1 ? parts[1] : ""
    state, state_err := load_state(allocator)
    if state_err != .None {
        fmt.printf("%sPackage %s not installed.%s\n", COLOR_RED, pkg_name, COLOR_RESET)
        return 1
    }
    defer delete_state(&state, allocator)
    if _, ok := state[pkg_name]; !ok {
        fmt.printf("%sPackage %s not installed.%s\n", COLOR_RED, pkg_name, COLOR_RESET)
        return 1
    }
    if version != "" {
        if err := switch_version(allocator, pkg_name, version); err != .None {
            return 1
        }
    }
    backend_args: [dynamic]string
    defer {
        for arg in backend_args {
            delete(arg)
        }
        delete(backend_args)
    }
    append(&backend_args, BACKEND_PATH)
    append(&backend_args, "run")
    append(&backend_args, pkg_name)
    append(&backend_args, bin)
    for arg in extra_args {
        append(&backend_args, arg)
    }
    code, _ := run_command(backend_args[:])
    return code
}

build :: proc(allocator: mem.Allocator, name: string) -> Error {
    log_to_file("INFO", fmt.tprintf("Building %s", name))
    if !os.exists("info.hk") || !os.exists("wrapper") || !os.exists("contents") {
        return .InvalidArgs
    }
    output_file := fmt.tprintf("%s.hpm", name)
    defer delete(output_file)
    build_args := []string{"tar", "-I", "zstd", "-cf", output_file, "."}
    code, run_err := run_command(build_args[:])
    if code != 0 || run_err != .None {
        return .BackendFailed
    }
    fmt.printf("%s✔ Built %s.hpm successfully.%s\n", COLOR_GREEN, name, COLOR_RESET)
    return .None
}

search :: proc(allocator: mem.Allocator, query: string) -> Error {
    repo, repo_err := load_repo(allocator)
    if repo_err != .None {
        return repo_err
    }
    defer deinit_repo(&repo, allocator)
    results: [dynamic]struct {name: string, ver: string, desc: string}
    defer {
        for res in results {
            delete(res.name)
            delete(res.ver)
            delete(res.desc)
        }
        delete(results)
    }
    q_lower := strings.to_lower(query, allocator)
    defer delete(q_lower)
    for name, pkg in repo {
        name_lower := strings.to_lower(name, allocator)
        desc_lower := strings.to_lower(pkg.description, allocator)
        if strings.contains(name_lower, q_lower) || strings.contains(desc_lower, q_lower) {
            if len(pkg.versions) > 0 {
                latest_ver := pkg.versions[0].version // Assume sorted or find max
                for v in pkg.versions[1:] {
                    if compare_versions(v.version, latest_ver) > 0 {
                        latest_ver = v.version
                    }
                }
                short_desc := pkg.description[:min(len(pkg.description), 50)]
                append(&results, struct {name: string, ver: string, desc: string}{name = name, ver = latest_ver, desc = short_desc})
            }
        }
        delete(name_lower)
        delete(desc_lower)
    }
    if len(results) == 0 {
        fmt.printf("%sNo results found for '%s'.%s\n", COLOR_YELLOW, query, COLOR_RESET)
        return .None
    }
    fmt.printf("%sSearch results for '%s':%s\n", COLOR_BLUE, query, COLOR_RESET)
    fmt.printf("%sName%s\t%sVersion%s\t%sDescription%s\n", COLOR_CYAN, COLOR_RESET, COLOR_CYAN, COLOR_RESET, COLOR_CYAN, COLOR_RESET)
    for res in results {
        fmt.printf("%s%s%s\t%s%s%s\t%s\n", COLOR_MAGENTA, res.name, COLOR_RESET, COLOR_GREEN, res.ver, COLOR_RESET, res.desc)
    }
    return .None
}

info :: proc(allocator: mem.Allocator, pkg_name: string) -> Error {
    repo, repo_err := load_repo(allocator)
    if repo_err != .None {
        return repo_err
    }
    defer deinit_repo(&repo, allocator)
    pkg, ok := repo[pkg_name]
    if !ok {
        return .PackageNotFound
    }
    state, state_err := load_state(allocator)
    if state_err != .None {
        return state_err
    }
    defer delete_state(&state, allocator)
    installed_ver := ""
    pinned := false
    if vers, sok := state[pkg_name]; sok && len(vers) > 0 {
        current_link := fmt.tprintf("%s%s/current", STORE_PATH, pkg_name)
        defer delete(current_link)
        target, rok := readlink(current_link, allocator)
        if rok {
            installed_ver = filepath.base(target)
            pinned = vers[installed_ver].pinned
            delete(target)
        }
    }
    fmt.printf("%sPackage:%s %s%s%s\n", COLOR_BLUE, COLOR_RESET, COLOR_CYAN, pkg_name, COLOR_RESET)
    fmt.printf("%sAuthor:%s %s\n", COLOR_BLUE, COLOR_RESET, pkg.author)
    fmt.printf("%sLicense:%s %s\n", COLOR_BLUE, COLOR_RESET, pkg.license)
    fmt.printf("%sDescription:%s %s\n", COLOR_BLUE, COLOR_RESET, pkg.description)
    fmt.printf("%sDependencies:%s ", COLOR_BLUE, COLOR_RESET)
    for v in pkg.versions {
        if v.version == installed_ver || installed_ver == "" {
            for dep, req in v.deps {
                fmt.printf("%s%s%s (%s) ", COLOR_MAGENTA, dep, COLOR_RESET, req)
            }
            break
        }
    }
    fmt.println()
    fmt.printf("%sAvailable versions:%s ", COLOR_BLUE, COLOR_RESET)
    for v in pkg.versions {
        fmt.printf("%s%s%s ", COLOR_GREEN, v.version, COLOR_RESET)
    }
    fmt.println()
    if installed_ver != "" {
        fmt.printf("%sInstalled:%s Yes (%s%s%s)\n", COLOR_BLUE, COLOR_RESET, COLOR_CYAN, installed_ver, COLOR_RESET)
        fmt.printf("%sPinned:%s %v\n", COLOR_BLUE, COLOR_RESET, pinned)
    } else {
        fmt.printf("%sInstalled:%s No\n", COLOR_BLUE, COLOR_RESET)
    }
    return .None
}

list_installed :: proc(allocator: mem.Allocator) -> Error {
    state, state_err := load_state(allocator)
    if state_err != .None {
        return state_err
    }
    defer delete_state(&state, allocator)
    if len(state) == 0 {
        fmt.printf("%sNo packages installed.%s\n", COLOR_YELLOW, COLOR_RESET)
        return .None
    }
    fmt.printf("%sInstalled packages:%s\n", COLOR_BLUE, COLOR_RESET)
    fmt.printf("%sPackage%s\t%sVersion%s\t%sInstall Date%s\t%sPinned%s\n", COLOR_CYAN, COLOR_RESET, COLOR_CYAN, COLOR_RESET, COLOR_CYAN, COLOR_RESET, COLOR_CYAN, COLOR_RESET)
    for pkg, vers in state {
        current_link := fmt.tprintf("%s%s/current", STORE_PATH, pkg)
        defer delete(current_link)
        target, ok := readlink(current_link, allocator)
        if ok {
            ver := filepath.base(target)
            date := vers[ver].date
            date_str := fmt.tprintf("%v", date)
            pinned := vers[ver].pinned
            fmt.printf("%s%s%s\t%s%s%s\t%s\t%v\n", COLOR_MAGENTA, pkg, COLOR_RESET, COLOR_GREEN, ver, COLOR_RESET, date_str, pinned)
            delete(target)
        }
    }
    return .None
}

clean_cache :: proc(allocator: mem.Allocator) -> Error {
    log_to_file("INFO", "Cleaning cache")
    dir, err := os.open(CACHE_PATH)
    if err != os.ERROR_NONE {
        return .CleanFailed
    }
    defer os.close(dir)
    files, _ := os.read_dir(dir, -1, allocator)
    defer delete(files)
    for file in files {
        if strings.has_suffix(file.name, ".hpm") {
            full_path := filepath.join({CACHE_PATH, file.name})
            os.remove(full_path)
            delete(full_path)
        }
    }
    fmt.printf("%s✔ Cache cleaned.%s\n", COLOR_GREEN, COLOR_RESET)
    return .None
}

pin :: proc(allocator: mem.Allocator, pkg_name: string, version: string) -> Error {
    lock_err := acquire_lock()
    if lock_err != .None {
        return lock_err
    }
    defer release_lock()
    state, state_err := load_state(allocator)
    if state_err != .None {
        return state_err
    }
    defer delete_state(&state, allocator)
    if vers, ok := state[pkg_name]; ok {
        if info, vok := vers[version]; vok {
            info.pinned = true
            vers[version] = info
            state[pkg_name] = vers
            save_state(&state, allocator)
            fmt.printf("%s✔ Pinned %s@%s.%s\n", COLOR_GREEN, pkg_name, version, COLOR_RESET)
            return .None
        } else {
            return .VersionNotFound
        }
    } else {
        return .PackageNotFound
    }
}

unpin :: proc(allocator: mem.Allocator, pkg_name: string) -> Error {
    lock_err := acquire_lock()
    if lock_err != .None {
        return lock_err
    }
    defer release_lock()
    state, state_err := load_state(allocator)
    if state_err != .None {
        return state_err
    }
    defer delete_state(&state, allocator)
    if vers, ok := state[pkg_name]; ok {
        current_link := fmt.tprintf("%s%s/current", STORE_PATH, pkg_name)
        defer delete(current_link)
        target, rok := readlink(current_link, allocator)
        if rok {
            ver := filepath.base(target)
            if info, vok := vers[ver]; vok {
                info.pinned = false
                vers[ver] = info
                state[pkg_name] = vers
                save_state(&state, allocator)
                fmt.printf("%s✔ Unpinned %s.%s\n", COLOR_GREEN, pkg_name, COLOR_RESET)
                delete(target)
                return .None
            } else {
                delete(target)
                return .VersionNotFound
            }
        } else {
            return .VersionNotFound
        }
    } else {
        return .PackageNotFound
    }
}

outdated :: proc(allocator: mem.Allocator) -> Error {
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
    outdated_list: [dynamic]struct {pkg: string, current: string, latest: string}
    defer delete(outdated_list)
    for pkg_name in state {
        current_link := fmt.tprintf("%s%s/current", STORE_PATH, pkg_name)
        defer delete(current_link)
        target, ok := readlink(current_link, allocator)
        if !ok {
            continue
        }
        current_ver := filepath.base(target)
        delete(target)
        pkg, okk := repo[pkg_name]
        if !okk {
            continue
        }
        sorted_versions: [dynamic]string
        defer {
            for v in sorted_versions {
                delete(v)
            }
            delete(sorted_versions)
        }
        for v in pkg.versions {
            append(&sorted_versions, strings.clone(v.version))
        }
        sort.sort(sort.Interface{
            collection = &sorted_versions,
            len = proc(it: sort.Interface) -> int { return len((^[dynamic]string)(it.collection)^) },
                  less = proc(it: sort.Interface, i, j: int) -> bool {
                      arr := (^[dynamic]string)(it.collection)^
                      return compare_versions(arr[i], arr[j]) > 0
                  },
                  swap = proc(it: sort.Interface, i, j: int) {
                      arr := (^[dynamic]string)(it.collection)^
                      arr[i], arr[j] = arr[j], arr[i]
                  },
        })
        if len(sorted_versions) == 0 {
            continue
        }
        latest_ver := sorted_versions[0]
        if compare_versions(latest_ver, current_ver) > 0 {
            append(&outdated_list, struct {pkg: string, current: string, latest: string}{pkg_name, current_ver, latest_ver})
        }
    }
    if len(outdated_list) == 0 {
        fmt.printf("%sAll packages are up to date.%s\n", COLOR_GREEN, COLOR_RESET)
        return .None
    }
    fmt.printf("%sOutdated packages:%s\n", COLOR_YELLOW, COLOR_RESET)
    fmt.printf("%sPackage%s\t%sCurrent%s\t%sLatest%s\n", COLOR_CYAN, COLOR_RESET, COLOR_CYAN, COLOR_RESET, COLOR_CYAN, COLOR_RESET)
    for item in outdated_list {
        fmt.printf("%s%s%s\t%s%s%s\t%s%s%s\n", COLOR_MAGENTA, item.pkg, COLOR_RESET, COLOR_RED, item.current, COLOR_RESET, COLOR_GREEN, item.latest, COLOR_RESET)
    }
    return .None
}

deps :: proc(allocator: mem.Allocator, pkg_spec: string) -> Error {
    repo, repo_err := load_repo(allocator)
    if repo_err != .None {
        return repo_err
    }
    defer deinit_repo(&repo, allocator)
    parts := strings.split(pkg_spec, "@")
    defer delete(parts)
    pkg_name := parts[0]
    req := len(parts) > 1 ? fmt.tprintf("=%s", parts[1]) : ""
    chosen: map[string]string
    order: [dynamic]struct {pkg: string, ver: string}
    defer {
        for k, v in chosen {
            delete(k, allocator)
            delete(v, allocator)
        }
        delete(chosen)
        delete(order)
    }
    res_err := resolve_deps_iterative(allocator, &repo, pkg_name, req, &chosen, &order)
    if res_err != .None {
        return res_err
    }
    fmt.printf("%sDependency tree for %s:%s\n", COLOR_BLUE, pkg_spec, COLOR_RESET)
    for item in order {
        fmt.printf("- %s%s@%s%s\n", COLOR_CYAN, item.pkg, item.ver, COLOR_RESET)
    }
    return .None
}
