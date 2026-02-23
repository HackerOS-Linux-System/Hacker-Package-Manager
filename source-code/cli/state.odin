package hpm
import "core:fmt"
import "core:os"
import "core:mem"
import "core:strings"
import "core:encoding/json"
import "core:time"
import "core:path/filepath"
VersionInfo :: struct {
    checksum: string,
    date: time.Time,
    pinned: bool,
}
StatePackages :: map[string]map[string]VersionInfo
load_state :: proc(allocator: mem.Allocator) -> (StatePackages, Error) {
    data, ok := os.read_entire_file(STATE_PATH, allocator)
    if !ok {
        return make(StatePackages, allocator), .None // Empty state is ok
    }
    defer delete(data)
    full_state: struct { packages: StatePackages }
    err := json.unmarshal(data, &full_state, allocator = allocator)
    if err != nil {
        return {}, .StateLoadFailed
    }
    return full_state.packages, .None
}
save_state :: proc(packages: ^StatePackages, allocator: mem.Allocator) -> Error {
    full_state := struct { packages: StatePackages }{packages^}
    data, merr := json.marshal(full_state, allocator = allocator)
    if merr != nil {
        return .StateLoadFailed // Marshal failed
    }
    defer delete(data)
    os.write_entire_file(STATE_TMP_PATH, data)
    if os.rename(STATE_TMP_PATH, STATE_PATH) != os.ERROR_NONE {
        return .StateLoadFailed
    }
    return .None
}
delete_state :: proc(state: ^StatePackages, allocator: mem.Allocator) {
    for key, val in state^ {
        for vkey, vinfo in val {
            delete(vkey, allocator)
            delete(vinfo.checksum, allocator)
        }
        delete(val)
        delete(key, allocator)
    }
    delete(state^)
}
get_installed :: proc(allocator: mem.Allocator, packages: ^StatePackages) -> (map[string]string, Error) {
    installed := make(map[string]string, allocator)
    for pkg in packages^ {
        current_link := fmt.tprintf("%s%s/current", STORE_PATH, pkg)
        defer delete(current_link)
        stat, err_stat := os.stat(current_link)
        if err_stat != os.ERROR_NONE || !os.S_ISLNK(u32(stat.mode)) {
            continue
        }
        target, ok := readlink(current_link, allocator)
        if ok {
            ver := filepath.base(target)
            installed[pkg] = ver
            delete(target)
        }
    }
    return installed, .None
}
