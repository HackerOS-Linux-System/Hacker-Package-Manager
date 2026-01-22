package hpm
import "core:fmt"
import "core:os"
import "core:mem"
import "core:strings"
import "core:encoding/json"
import "core:crypto/sha2"
import "core:path/filepath"
import "core:strconv"
import "core:sort"
import "core:sys/linux"
VERSION :: "0.5"
STORE_PATH :: "/usr/lib/HackerOS/hpm/store/"
BACKEND_PATH :: "/usr/bin/hpm-backend" // Zakładam, że backend jest zainstalowany jako oddzielna binarka; dostosuj jeśli potrzeba
REPO_JSON_URL :: "https://raw.githubusercontent.com/HackerOS-Linux-System/Hacker-Package-Manager/main/Community/repo.json"
VERSION_URL :: "https://raw.githubusercontent.com/HackerOS-Linux-System/Hacker-Package-Manager/main/Community/hpm-version.hacker"
LOCAL_VERSION_FILE :: "/usr/lib/HackerOS/hpm/version.json"
RELEASES_BASE :: "https://github.com/HackerOS-Linux-System/Hacker-Package-Manager/releases/download/v"
STATE_PATH :: "/var/lib/hpm/state.json"
// Kolory ANSI dla ładnego wyglądu
COLOR_GREEN :: "\033[1;32m"
COLOR_YELLOW :: "\033[1;33m"
COLOR_RED :: "\033[1;31m"
COLOR_RESET :: "\033[0m"
WIFEXITED :: proc "contextless" (status: i32) -> bool { return ((status) & 0o177) == 0 }
WEXITSTATUS :: proc "contextless" (status: i32) -> i32 { return ((status) >> 8) & 0x000000ff }
run_command :: proc(args: []string) -> int {
    if len(args) == 0 {
        return 1
    }
    path_env := os.get_env("PATH")
    exec_path: string
    if filepath.is_abs(args[0]) || strings.contains_rune(args[0], '/') {
        exec_path = args[0]
    } else {
        paths := strings.split(path_env, ":")
        defer delete(paths)
        for p in paths {
            candidate := filepath.join({p, args[0]})
            defer delete(candidate)
            if os.exists(candidate) {
                stat, err := os.stat(candidate)
                if err == os.ERROR_NONE && (stat.mode & os.S_IXUSR != 0) {
                    exec_path = candidate
                    break
                }
            }
        }
        if exec_path == "" {
            return 127 // Command not found
        }
    }
    args_c: [dynamic]cstring
    defer delete(args_c)
    reserve(&args_c, len(args) + 1)
    for arg in args {
        append(&args_c, strings.clone_to_cstring(arg, context.temp_allocator))
    }
    append(&args_c, nil)
    exec_path_c := strings.clone_to_cstring(exec_path, context.temp_allocator)
    defer delete(exec_path_c)
    res := linux.syscall(linux.SYS_fork)
    fork_err := linux.Errno(0)
        if res < 0 {
            fork_err = linux.Errno(-res)
        }
        if fork_err != linux.Errno.NONE {
            return 1
        }
        pid := i32(res)
        if pid == 0 {
            // Child process
            exec_res := linux.syscall(linux.SYS_execve, uintptr(rawptr(exec_path_c)), uintptr(rawptr(raw_data(args_c))), uintptr(0))
            exec_err := linux.Errno(0)
            if exec_res < 0 {
                exec_err = linux.Errno(-exec_res)
            }
            // If execve fails
            linux.syscall(linux.SYS_exit_group, uintptr(1))
        } else if pid > 0 {
            // Parent
            status: i32
            wait_res := linux.syscall(linux.SYS_wait4, uintptr(pid), uintptr(rawptr(&status)), uintptr(0), uintptr(0))
            wait_err := linux.Errno(0)
            if wait_res < 0 {
                wait_err = linux.Errno(-wait_res)
            }
            if wait_err != linux.Errno.NONE || wait_res == -1 {
                return 1
            }
            if WIFEXITED(status) {
                return int(WEXITSTATUS(status))
            } else {
                return 1
            }
        } else {
            // Fork failed
            return 1
        }
        return 1 // Fallback return
}
Manifest :: struct {
    bins: [dynamic]string,
}
load_info :: proc(allocator: mem.Allocator, path: string) -> Manifest {
    info_path := filepath.join([]string{path, "info.hk"}, allocator)
    defer delete(info_path, allocator)
    data, ok := os.read_entire_file(info_path, allocator)
    if !ok {
        fmt.printf("%sFailed to read info.hk.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    defer delete(data, allocator)
    m: struct { bins: []string }
    err := json.unmarshal(data, &m, allocator=allocator)
    if err != nil {
        fmt.printf("%sFailed to parse info.hk.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    bins := make([dynamic]string, len(m.bins), allocator)
    for b, i in m.bins {
        bins[i] = strings.clone(b, allocator)
    }
    delete(m.bins)
    return Manifest { bins = bins }
}
deinit_manifest :: proc(m: ^Manifest) {
    for b in m.bins {
        delete(b)
    }
    delete(m.bins)
}
main :: proc() {
    arena: mem.Arena
    backing := make([]u8, 8 * mem.Megabyte)
    mem.arena_init(&arena, backing)
    defer delete(backing)
    allocator := mem.arena_allocator(&arena)
    context.allocator = allocator
    args := os.args[1:]
    if len(args) < 1 {
        print_help()
        return
    }
    command := args[0]
    switch command {
        case "refresh":
            refresh(allocator)
        case "install":
            if len(args) < 2 {
                fmt.printf("%sUsage: hpm install <package>@[version]%s\n", COLOR_RED, COLOR_RESET)
                return
            }
            install(allocator, args[1])
        case "remove":
            if len(args) < 2 {
                fmt.printf("%sUsage: hpm remove <package>@[version]%s\n", COLOR_RED, COLOR_RESET)
                return
            }
            remove(allocator, args[1])
        case "update":
            update(allocator)
        case "switch":
            if len(args) < 3 {
                fmt.printf("%sUsage: hpm switch <package> <version>%s\n", COLOR_RED, COLOR_RESET)
                return
            }
            switch_version(allocator, args[1], args[2])
        case "upgrade":
            upgrade(allocator)
        case "run":
            if len(args) < 2 {
                fmt.printf("%sUsage: hpm run <package>@[version] <bin> [args...]%s\n", COLOR_RED, COLOR_RESET)
                return
            }
            run_code := run_tool(allocator, args[1:])
            os.exit(run_code)
        case "build":
            if len(args) < 2 {
                fmt.printf("%sUsage: hpm build <package_name>%s\n", COLOR_RED, COLOR_RESET)
                return
            }
            build(allocator, args[1])
        case:
            print_help()
    }
}
print_help :: proc() {
    fmt.printf("%sHPM %s - Hacker Package Manager%s\n", COLOR_GREEN, VERSION, COLOR_RESET)
    fmt.println("Usage: hpm <command> [args]")
    fmt.println("Commands:")
    fmt.println(" refresh Refresh package index")
    fmt.println(" install <pkg> Install package (with optional @version)")
    fmt.println(" remove <pkg> Remove package (with optional @version)")
    fmt.println(" update Update all installed packages")
    fmt.println(" switch <pkg> <ver> Switch to specific version")
    fmt.println(" upgrade Upgrade HPM itself")
    fmt.println(" run <pkg>@[ver] <bin> [args] Run tool from package")
    fmt.println(" build <name> Build .hpm package from current directory")
}
download_file :: proc(allocator: mem.Allocator, url: string, path: string) {
    fmt.printf("%sDownloading %s...%s\n", COLOR_YELLOW, url, COLOR_RESET)
    args := []string{"curl", "-L", "-o", path, url}
    code := run_command(args[:])
    if code != 0 {
        fmt.printf("%sDownload failed.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    fmt.printf("%sDownload complete.%s\n", COLOR_GREEN, COLOR_RESET)
}
compute_sha256 :: proc(allocator: mem.Allocator, path: string) -> string {
    data, ok := os.read_entire_file(path, allocator)
    if !ok {
        fmt.printf("%sFailed to read file for SHA256.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    defer delete(data)
    ctx: sha2.Context_256
    sha2.init_256(&ctx)
    sha2.update(&ctx, data)
    hash: [sha2.DIGEST_SIZE_256]u8
    sha2.final(&ctx, hash[:])
    sb: strings.Builder
    strings.builder_init(&sb, allocator)
    defer strings.builder_destroy(&sb)
    for b in hash {
        fmt.sbprintf(&sb, "{:02x}", b)
    }
    return strings.to_string(sb)
}
refresh :: proc(allocator: mem.Allocator) {
    fmt.printf("%sRefreshing package index...%s\n", COLOR_YELLOW, COLOR_RESET)
    temp_path := "/tmp/repo.json"
    download_file(allocator, REPO_JSON_URL, temp_path)
    os.rename(temp_path, "/usr/lib/HackerOS/hpm/repo.json")
    fmt.printf("%sPackage index refreshed.%s\n", COLOR_GREEN, COLOR_RESET)
}
compare_versions :: proc(a: string, b: string) -> i32 {
    parts_a := strings.split_multi(a, []string{".", "-"}, context.temp_allocator)
    parts_b := strings.split_multi(b, []string{".", "-"}, context.temp_allocator)
    max_len := max(len(parts_a), len(parts_b))
    for i in 0..<max_len {
        if i >= len(parts_a) { return -1 }
        if i >= len(parts_b) { return 1 }
        pa := parts_a[i]
        pb := parts_b[i]
        ia, oka := strconv.parse_int(pa)
        ib, okb := strconv.parse_int(pb)
        if oka && okb {
            if ia != ib { return ia > ib ? 1 : -1 }
        } else {
            if pa != pb { return pa > pb ? 1 : -1 }
        }
    }
    return 0
}
satisfies :: proc(ver: string, req: string) -> bool {
    if req == "" { return true }
    if strings.has_prefix(req, ">=") {
        req_ver := strings.trim_space(req[2:])
        return compare_versions(ver, req_ver) >= 0
    } else if strings.has_prefix(req, ">") {
        req_ver := strings.trim_space(req[1:])
        return compare_versions(ver, req_ver) > 0
    } else if strings.has_prefix(req, "=") {
        req_ver := strings.trim_space(req[1:])
        return ver == req_ver
    } else {
        return ver == req
    }
}
Repo :: map[string]struct {
    versions: [dynamic]struct {
        version: string,
        url: string,
        sha256: string,
        deps: map[string]string,
    },
}
load_repo :: proc(allocator: mem.Allocator) -> Repo {
    repo_path := "/usr/lib/HackerOS/hpm/repo.json"
    data, ok := os.read_entire_file(repo_path, allocator)
    if !ok {
        fmt.printf("%sRepo not found. Run 'hpm refresh' first.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    defer delete(data)
    repo: Repo
    err := json.unmarshal(data, &repo, allocator = allocator)
    if err != nil {
        fmt.printf("%sFailed to parse repo.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    return repo
}
StatePackages :: map[string]map[string]string // pkg -> ver -> checksum
load_state :: proc(allocator: mem.Allocator) -> StatePackages {
    data, ok := os.read_entire_file(STATE_PATH, allocator)
    if !ok {
        return {}
    }
    defer delete(data)
    full_state: struct { packages: StatePackages }
    err := json.unmarshal(data, &full_state, allocator = allocator)
    if err != nil {
        fmt.printf("%sFailed to parse state.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    return full_state.packages
}
save_state :: proc(packages: StatePackages, allocator: mem.Allocator) {
    full_state := struct { packages: StatePackages }{packages}
    data, err := json.marshal(full_state, allocator = allocator)
    if err != nil {
        fmt.printf("%sJSON marshal failed.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    defer delete(data)
    os.write_entire_file(STATE_PATH, data)
}
choose_version :: proc(repo: Repo, pkg_name: string, req: string, chosen: ^map[string]string) {
    if _, ok := chosen[pkg_name]; ok {
        existing_ver := chosen[pkg_name]
        if !satisfies(existing_ver, req) {
            fmt.printf("%sVersion conflict for %s: %s does not satisfy %s%s\n", COLOR_RED, pkg_name, existing_ver, req, COLOR_RESET)
            os.exit(1)
        }
        return
    }
    pkg, ok := repo[pkg_name]
    if !ok {
        fmt.printf("%sPackage %s not found.%s\n", COLOR_RED, pkg_name, COLOR_RESET)
        os.exit(1)
    }
    compatible: [dynamic]string
    defer delete(compatible)
    for v in pkg.versions {
        if satisfies(v.version, req) {
            append(&compatible, v.version)
        }
    }
    if len(compatible) == 0 {
        fmt.printf("%sNo version for %s satisfies %s%s\n", COLOR_RED, pkg_name, req, COLOR_RESET)
        os.exit(1)
    }
    // Sort and pick max
    sort_versions :: proc(versions: []string) -> []string {
        sorted := make([]string, len(versions), context.temp_allocator)
        copy(sorted, versions)
        for i in 0..<len(sorted)-1 {
            for j in i+1..<len(sorted) {
                if compare_versions(sorted[i], sorted[j]) > 0 {
                    sorted[i], sorted[j] = sorted[j], sorted[i]
                }
            }
        }
        return sorted
    }
    sorted := sort_versions(compatible[:])
    chosen[pkg_name] = sorted[len(sorted)-1]
}
resolve_deps :: proc(allocator: mem.Allocator, repo: Repo, pkg_name: string, req: string, chosen: ^map[string]string, visiting: ^map[string]bool, order: ^[dynamic]struct {pkg: string, ver: string}) {
    if visiting[pkg_name] {
        fmt.printf("%sDependency cycle detected involving %s%s\n", COLOR_RED, pkg_name, COLOR_RESET)
        os.exit(1)
    }
    visiting[pkg_name] = true
    choose_version(repo, pkg_name, req, chosen)
    ver := chosen[pkg_name]
    pkg := repo[pkg_name]
    ver_obj: struct {version: string, url: string, sha256: string, deps: map[string]string}
    for v in pkg.versions {
        if v.version == ver {
            ver_obj = v
            break
        }
    }
    for dep, dep_req in ver_obj.deps {
        resolve_deps(allocator, repo, dep, dep_req, chosen, visiting, order)
    }
    append(order, struct {pkg: string, ver: string}{pkg_name, ver})
    visiting[pkg_name] = false
}
readlink :: proc(path: string, allocator: mem.Allocator) -> (string, bool) {
    MAX_PATH :: 4096
    buf := make([]u8, MAX_PATH, allocator)
    defer delete(buf, allocator)
    path_c := strings.clone_to_cstring(path, context.temp_allocator)
    defer delete(path_c)
    res := linux.syscall(linux.SYS_readlink, uintptr(rawptr(path_c)), uintptr(rawptr(raw_data(buf))), uintptr(MAX_PATH))
    err := linux.Errno(0)
    if res < 0 {
        err = linux.Errno(-res)
    }
    if err != linux.Errno.NONE || res < 0 {
        return "", false
    }
    return string(buf[:res]), true
}
get_installed :: proc(allocator: mem.Allocator, packages: StatePackages) -> map[string]string {
    installed: map[string]string
    for pkg in packages {
        current_link := fmt.aprintf("%s%s/current", STORE_PATH, pkg)
        defer delete(current_link, allocator)
        stat, err := os.lstat(current_link)
        if err == os.ERROR_NONE && os.S_ISLNK(u32(stat.mode)) {
            target, ok := readlink(current_link, allocator)
            if ok {
                ver := filepath.base(target)
                installed[pkg] = ver
                delete(target, allocator)
            }
        }
    }
    return installed
}
install :: proc(allocator: mem.Allocator, pkg_spec: string) {
    fmt.printf("%sInstalling %s...%s\n", COLOR_YELLOW, pkg_spec, COLOR_RESET)
    repo := load_repo(allocator)
    defer {
        for key, val in repo {
            delete(key)
            for v in val.versions {
                delete(v.version)
                delete(v.url)
                delete(v.sha256)
                for dkey, dval in v.deps {
                    delete(dkey)
                    delete(dval)
                }
                delete(v.deps)
            }
            delete(val.versions)
        }
        delete(repo)
    }
    packages := load_state(allocator)
    installed := get_installed(allocator, packages)
    defer delete(installed)
    parts := strings.split(pkg_spec, "@", allocator)
    defer delete(parts)
    pkg_name := parts[0]
    requested_ver := len(parts) > 1 ? parts[1] : ""
    req := requested_ver
    chosen: map[string]string
    visiting: map[string]bool
    order: [dynamic]struct {pkg: string, ver: string}
    defer {
        delete(chosen)
        delete(visiting)
        delete(order)
    }
    resolve_deps(allocator, repo, pkg_name, req, &chosen, &visiting, &order)
    for item in order {
        p, v := item.pkg, item.ver
        if inst_ver, ok := installed[p]; !ok || !satisfies(inst_ver, v) {
            install_single(allocator, p, v, repo)
        }
    }
    fmt.printf("%s%s and dependencies installed.%s\n", COLOR_GREEN, pkg_spec, COLOR_RESET)
}
install_single :: proc(allocator: mem.Allocator, package_name: string, version: string, repo: Repo) {
    pkg := repo[package_name]
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
        fmt.printf("%sVersion %s not found for %s%s\n", COLOR_RED, version, package_name, COLOR_RESET)
        os.exit(1)
    }
    pkg_url := ver_obj.url
    expected_sha := ver_obj.sha256
    pkg_path := fmt.aprintf("%s%s/%s", STORE_PATH, package_name, version)
    defer delete(pkg_path)
    current_link := fmt.aprintf("%s%s/current", STORE_PATH, package_name)
    defer delete(current_link)
    if os.exists(pkg_path) {
        fmt.printf("%sAlready installed %s@%s%s\n", COLOR_GREEN, package_name, version, COLOR_RESET)
        return
    }
    temp_archive := fmt.aprintf("/tmp/%s-%s.hpm", package_name, version)
    defer delete(temp_archive)
    temp_extract := fmt.aprintf("%s.tmp", pkg_path)
    defer delete(temp_extract)
    os.remove_directory(temp_extract)
    os.make_directory(temp_extract, 0o755)
    download_file(allocator, pkg_url, temp_archive)
    if expected_sha != "" {
        computed_sha := compute_sha256(allocator, temp_archive)
        if computed_sha != expected_sha {
            os.remove_directory(temp_extract)
            fmt.printf("%sSHA256 mismatch for %s%s\n", COLOR_RED, package_name, COLOR_RESET)
            os.exit(1)
        }
    }
    unpack_args := []string{"tar", "-I", "zstd", "-xf", temp_archive, "-C", temp_extract}
    code := run_command(unpack_args[:])
    if code != 0 {
        fmt.printf("%sUnpack failed.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    checksum := expected_sha != "" ? expected_sha : "none"
    backend_args := []string{BACKEND_PATH, "install", package_name, version, temp_extract, checksum}
    code = run_command(backend_args[:])
    if code != 0 {
        fmt.printf("%sBackend install failed.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    os.rename(temp_extract, pkg_path)
    os.make_directory(filepath.dir(current_link), 0o755)
    if os.exists(current_link) {
        os.remove(current_link)
    }
    version_c := strings.clone_to_cstring(version, context.temp_allocator)
    defer delete(version_c)
    current_link_c := strings.clone_to_cstring(current_link, context.temp_allocator)
    defer delete(current_link_c)
    sym_res := linux.syscall(linux.SYS_symlink, uintptr(rawptr(version_c)), uintptr(rawptr(current_link_c)))
    sym_err := linux.Errno(0)
    if sym_res < 0 {
        sym_err = linux.Errno(-sym_res)
    }
    if sym_err != linux.Errno.NONE {
        fmt.printf("%sFailed to create symlink.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    manifest := load_info(allocator, pkg_path)
    defer deinit_manifest(&manifest)
    for bin in manifest.bins {
        wrapper_path := fmt.aprintf("/usr/bin/%s", bin)
        defer delete(wrapper_path)
        wrapper_content := fmt.aprintf("#!/bin/sh\nexec %s run %s %s \"$@\"\n", BACKEND_PATH, package_name, bin)
        defer delete(wrapper_content)
        os.write_entire_file(wrapper_path, transmute([]u8)wrapper_content)
        wrapper_path_c := strings.clone_to_cstring(wrapper_path, context.temp_allocator)
        defer delete(wrapper_path_c)
        chmod_res := linux.syscall(linux.SYS_chmod, uintptr(rawptr(wrapper_path_c)), uintptr(0o755))
        chmod_err := linux.Errno(0)
        if chmod_res < 0 {
            chmod_err = linux.Errno(-chmod_res)
        }
        if chmod_err != linux.Errno.NONE {
            fmt.printf("%sFailed to chmod wrapper.%s\n", COLOR_RED, COLOR_RESET)
            os.exit(1)
        }
    }
    fmt.printf("%sInstalled %s@%s%s\n", COLOR_GREEN, package_name, version, COLOR_RESET)
}
remove :: proc(allocator: mem.Allocator, pkg_spec: string) {
    fmt.printf("%sRemoving %s...%s\n", COLOR_YELLOW, pkg_spec, COLOR_RESET)
    packages := load_state(allocator)
    defer save_state(packages, allocator)
    parts := strings.split(pkg_spec, "@", allocator)
    defer delete(parts)
    pkg_name := parts[0]
    version := len(parts) > 1 ? parts[1] : ""
    if _, ok := packages[pkg_name]; !ok {
        fmt.printf("%sPackage %s not installed.%s\n", COLOR_RED, pkg_name, COLOR_RESET)
        return
    }
    vers_map := packages[pkg_name]
    current_link := fmt.aprintf("%s%s/current", STORE_PATH, pkg_name)
    defer delete(current_link)
    if version != "" {
        if _, ok := vers_map[version]; !ok {
            fmt.printf("%sVersion %s not installed.%s\n", COLOR_RED, version, COLOR_RESET)
            return
        }
        installed_path := fmt.aprintf("%s%s/%s", STORE_PATH, pkg_name, version)
        defer delete(installed_path)
        backend_args := []string{BACKEND_PATH, "remove", pkg_name, version, installed_path}
        run_command(backend_args[:])
        os.remove_directory(installed_path)
        stat, err := os.lstat(current_link)
        if err == os.ERROR_NONE && os.S_ISLNK(u32(stat.mode)) {
            target, ok := readlink(current_link, allocator)
            if ok {
                if filepath.base(target) == version {
                    os.remove(current_link)
                }
                delete(target, allocator)
            }
        }
        delete_key(&vers_map, version)
        if len(vers_map) == 0 {
            delete_key(&packages, pkg_name)
            os.remove_directory(fmt.aprintf("%s%s", STORE_PATH, pkg_name))
        }
    } else {
        for ver in vers_map {
            installed_path := fmt.aprintf("%s%s/%s", STORE_PATH, pkg_name, ver)
            defer delete(installed_path)
            backend_args := []string{BACKEND_PATH, "remove", pkg_name, ver, installed_path}
            run_command(backend_args[:])
            os.remove_directory(installed_path)
        }
        os.remove_directory(fmt.aprintf("%s%s", STORE_PATH, pkg_name))
        delete_key(&packages, pkg_name)
    }
    fmt.printf("%s%s removed.%s\n", COLOR_GREEN, pkg_spec, COLOR_RESET)
}
update :: proc(allocator: mem.Allocator) {
    fmt.printf("%sUpdating installed packages...%s\n", COLOR_YELLOW, COLOR_RESET)
    repo := load_repo(allocator)
    // defer cleanup repo...
    packages := load_state(allocator)
    for pkg_name in packages {
        current_link := fmt.aprintf("%s%s/current", STORE_PATH, pkg_name)
        defer delete(current_link)
        stat, err := os.lstat(current_link)
        if err != os.ERROR_NONE || !os.S_ISLNK(u32(stat.mode)) { continue }
        target, ok := readlink(current_link, allocator)
        if !ok { continue }
        current_ver := filepath.base(target)
        delete(target, allocator)
        pkg, okk := repo[pkg_name]
        if !okk { continue }
        versions := pkg.versions[:]
        sorted_versions: [dynamic]string
        for v in versions {
            append(&sorted_versions, v.version)
        }
        defer delete(sorted_versions)
        // Sort descending
        for i in 0..<len(sorted_versions)-1 {
            for j in i+1..<len(sorted_versions) {
                if compare_versions(sorted_versions[i], sorted_versions[j]) < 0 {
                    sorted_versions[i], sorted_versions[j] = sorted_versions[j], sorted_versions[i]
                }
            }
        }
        latest_ver := sorted_versions[0]
        if compare_versions(latest_ver, current_ver) > 0 {
            fmt.printf("%sUpdating %s from %s to %s%s\n", COLOR_YELLOW, pkg_name, current_ver, latest_ver, COLOR_RESET)
            remove(allocator, fmt.aprintf("%s@%s", pkg_name, current_ver))
            install_single(allocator, pkg_name, latest_ver, repo)
        }
    }
    fmt.printf("%sUpdates complete.%s\n", COLOR_GREEN, COLOR_RESET)
}
switch_version :: proc(allocator: mem.Allocator, pkg_name: string, version: string) {
    fmt.printf("%sSwitching %s to %s...%s\n", COLOR_YELLOW, pkg_name, version, COLOR_RESET)
    packages := load_state(allocator)
    if _, ok := packages[pkg_name]; !ok {
        fmt.printf("%sPackage %s not installed.%s\n", COLOR_RED, pkg_name, COLOR_RESET)
        return
    }
    vers_map := packages[pkg_name]
    if _, ok := vers_map[version]; !ok {
        fmt.printf("%sVersion %s not installed.%s\n", COLOR_RED, version, COLOR_RESET)
        return
    }
    current_link := fmt.aprintf("%s%s/current", STORE_PATH, pkg_name)
    defer delete(current_link)
    if os.exists(current_link) {
        os.remove(current_link)
    }
    version_c := strings.clone_to_cstring(version, context.temp_allocator)
    defer delete(version_c)
    current_link_c := strings.clone_to_cstring(current_link, context.temp_allocator)
    defer delete(current_link_c)
    sym_res := linux.syscall(linux.SYS_symlink, uintptr(rawptr(version_c)), uintptr(rawptr(current_link_c)))
    sym_err := linux.Errno(0)
    if sym_res < 0 {
        sym_err = linux.Errno(-sym_res)
    }
    if sym_err != linux.Errno.NONE {
        fmt.printf("%sFailed to switch version.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    fmt.printf("%sSwitched %s to %s.%s\n", COLOR_GREEN, pkg_name, version, COLOR_RESET)
}
upgrade :: proc(allocator: mem.Allocator) {
    fmt.printf("%sChecking for HPM upgrade...%s\n", COLOR_YELLOW, COLOR_RESET)
    temp_version_file := "/tmp/hpm-version.hacker"
    download_file(allocator, VERSION_URL, temp_version_file)
    remote_raw, ok := os.read_entire_file(temp_version_file, allocator)
    if !ok {
        fmt.printf("%sFailed to read version file.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    defer delete(remote_raw)
    rep, _ := strings.replace_all(string(remote_raw), "[]", "")
    remote_version := strings.trim_space(rep)
    local_data, lok := os.read_entire_file(LOCAL_VERSION_FILE, allocator)
    local_version := "0.0"
    if lok {
        defer delete(local_data)
        lstate: struct {version: string}
        json.unmarshal(local_data, &lstate)
        local_version = lstate.version
    }
    if compare_versions(remote_version, local_version) > 0 {
        fmt.printf("%sUpgrading HPM to %s...%s\n", COLOR_YELLOW, remote_version, COLOR_RESET)
        hpm_url := fmt.aprintf("%s%s/hpm", RELEASES_BASE, remote_version)
        defer delete(hpm_url)
        download_file(allocator, hpm_url, "/usr/bin/hpm")
        hpm_path := "/usr/bin/hpm"
        hpm_path_c := strings.clone_to_cstring(hpm_path, context.temp_allocator)
        defer delete(hpm_path_c)
        chmod_res := linux.syscall(linux.SYS_chmod, uintptr(rawptr(hpm_path_c)), uintptr(0o755))
        chmod_err := linux.Errno(0)
        if chmod_res < 0 {
            chmod_err = linux.Errno(-chmod_res)
        }
        if chmod_err != linux.Errno.NONE {
            fmt.printf("%sFailed to chmod hpm.%s\n", COLOR_RED, COLOR_RESET)
            os.exit(1)
        }
        backend_url := fmt.aprintf("%s%s/backend", RELEASES_BASE, remote_version)
        defer delete(backend_url)
        download_file(allocator, backend_url, BACKEND_PATH)
        backend_path_c := strings.clone_to_cstring(BACKEND_PATH, context.temp_allocator)
        defer delete(backend_path_c)
        backend_chmod_res := linux.syscall(linux.SYS_chmod, uintptr(rawptr(backend_path_c)), uintptr(0o755))
        backend_chmod_err := linux.Errno(0)
        if backend_chmod_res < 0 {
            backend_chmod_err = linux.Errno(-backend_chmod_res)
        }
        if backend_chmod_err != linux.Errno.NONE {
            fmt.printf("%sFailed to chmod backend.%s\n", COLOR_RED, COLOR_RESET)
            os.exit(1)
        }
        new_version := struct {version: string}{remote_version}
        data, _ := json.marshal(new_version)
        defer delete(data)
        os.write_entire_file(LOCAL_VERSION_FILE, data)
        fmt.printf("%sUpgrade complete.%s\n", COLOR_GREEN, COLOR_RESET)
    } else {
        fmt.printf("%sHPM is up to date.%s\n", COLOR_GREEN, COLOR_RESET)
    }
}
run_tool :: proc(allocator: mem.Allocator, args: []string) -> int {
    if len(args) < 2 {
        fmt.printf("%sUsage: hpm run <package>@[version] <bin> [args...]%s\n", COLOR_RED, COLOR_RESET)
        return 1
    }
    package_spec := args[0]
    bin := args[1]
    extra_args := args[2:]
    parts := strings.split(package_spec, "@", allocator)
    defer delete(parts)
    pkg_name := parts[0]
    version := len(parts) > 1 ? parts[1] : ""
    backend_args: [dynamic]string
    defer delete(backend_args)
    append(&backend_args, BACKEND_PATH)
    append(&backend_args, "run")
    append(&backend_args, pkg_name)
    append(&backend_args, bin)
    for arg in extra_args {
        append(&backend_args, arg)
    }
    if version != "" {
        switch_version(allocator, pkg_name, version)
    }
    code := run_command(backend_args[:])
    return code
}
build :: proc(allocator: mem.Allocator, name: string) {
    fmt.printf("%sBuilding %s.hpm...%s\n", COLOR_YELLOW, name, COLOR_RESET)
    // Sprawdź wymagane pliki
    if !os.exists("info.hk") || !os.exists("wrapper") || !os.exists("contents") {
        fmt.printf("%sMust be in directory with info.hk, wrapper, and contents folder.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    output_file := fmt.aprintf("%s.hpm", name)
    defer delete(output_file)
    build_args := []string{"tar", "-I", "zstd", "-cf", output_file, "."}
    code := run_command(build_args[:])
    if code != 0 {
        fmt.printf("%sBuild failed.%s\n", COLOR_RED, COLOR_RESET)
        os.exit(1)
    }
    fmt.printf("%sBuilt %s.hpm successfully.%s\n", COLOR_GREEN, name, COLOR_RESET)
}
