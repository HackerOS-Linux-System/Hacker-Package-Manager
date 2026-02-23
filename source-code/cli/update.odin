package hpm

import "core:fmt"
import "core:os"
import "core:mem"
import "core:strings"
import "core:path/filepath"
import "core:sort"

update :: proc(allocator: mem.Allocator) -> Error {
    lock_err := acquire_lock()
    if lock_err != .None {
        return lock_err
    }
    defer release_lock()
    log_to_file("INFO", "Updating installed packages")
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
    err_save := save_state(&state, allocator)
    if err_save != .None {
        return err_save
    }
    updated_count := 0
    current_count := 0
    for pkg_name in state {
        current_link := fmt.tprintf("%s%s/current", STORE_PATH, pkg_name)
        defer delete(current_link, allocator)
        stat, err_stat := os.stat(current_link)
        if err_stat != os.ERROR_NONE || !os.S_ISLNK(u32(stat.mode)) {
            continue
        }
        target, ok := readlink(current_link, allocator)
        if !ok {
            continue
        }
        current_ver := filepath.base(target)
        delete(target, allocator)
        if state[pkg_name][current_ver].pinned {
            current_count += 1
            continue
        }
        pkg, okk := repo[pkg_name]
        if !okk {
            continue
        }
        sorted_versions: [dynamic]string
        defer delete(sorted_versions)
        for v in pkg.versions {
            append(&sorted_versions, strings.clone(v.version, allocator))
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
            fmt.printf("%s➤ Updating %s%s%s from %s%s%s to %s%s%s%s\n", COLOR_YELLOW, COLOR_CYAN, pkg_name, COLOR_RESET, COLOR_CYAN, current_ver, COLOR_RESET, COLOR_CYAN, latest_ver, COLOR_RESET, COLOR_RESET)
            rem_err := remove(allocator, fmt.tprintf("%s@%s", pkg_name, current_ver), true)
            if rem_err != .None {
                return rem_err
            }
            inst_err := install_single(allocator, pkg_name, latest_ver, &repo, &state)
            if inst_err != .None {
                return inst_err
            }
            updated_count += 1
        } else {
            current_count += 1
        }
    }
    fmt.printf("%s✔ Updates complete. Updated: %d, Already current: %d%s\n", COLOR_GREEN, updated_count, current_count, COLOR_RESET)
    err_save = save_state(&state, allocator)
    if err_save != .None {
        return err_save
    }
    return .None
}
