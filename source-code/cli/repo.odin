package hpm
import "core:fmt"
import "core:os"
import "core:mem"
import "core:strings"
import "core:encoding/json"
import "core:strconv"
import "core:sort"
RepoPackage :: struct {
    author: string,
    license: string,
    description: string,
    versions: [dynamic]struct {
        version: string,
        url: string,
        sha256: string,
        deps: map[string]string,
    },
}
Repo :: map[string]RepoPackage
load_repo :: proc(allocator: mem.Allocator) -> (Repo, Error) {
    repo_path := "/usr/lib/HackerOS/hpm/repo.json"
    data, ok := os.read_entire_file(repo_path, allocator)
    if !ok {
        print_error(.RepoLoadFailed)
        return {}, .RepoLoadFailed
    }
    defer delete(data)
    repo: Repo
    err := json.unmarshal(data, &repo, allocator = allocator)
    if err != nil {
        print_error(.RepoLoadFailed)
        return {}, .RepoLoadFailed
    }
    return repo, .None
}
deinit_repo :: proc(repo: ^Repo, allocator: mem.Allocator) {
    for key, val in repo^ {
        delete(key, allocator)
        delete(val.author, allocator)
        delete(val.license, allocator)
        delete(val.description, allocator)
        for i in 0..<len(val.versions) {
            v := val.versions[i]
            delete(v.version, allocator)
            delete(v.url, allocator)
            delete(v.sha256, allocator)
            for dk, dv in v.deps {
                delete(dk, allocator)
                delete(dv, allocator)
            }
            delete(v.deps)
        }
        delete(val.versions)
    }
    delete(repo^)
}
compare_versions :: proc(a: string, b: string) -> int {
    parts_a := strings.split_multi(a, {".", "-"})
    defer delete(parts_a, context.temp_allocator)
    parts_b := strings.split_multi(b, {".", "-"})
    defer delete(parts_b, context.temp_allocator)
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
        req_ver := strings.trim_prefix(req, ">=")
        return compare_versions(ver, req_ver) >= 0
    } else if strings.has_prefix(req, ">") {
        req_ver := strings.trim_prefix(req, ">")
        return compare_versions(ver, req_ver) > 0
    } else if strings.has_prefix(req, "=") {
        req_ver := strings.trim_prefix(req, "=")
        return ver == req_ver
    } else {
        return ver == req
    }
}
choose_version :: proc(repo: ^Repo, pkg_name: string, req: string, chosen: ^map[string]string, allocator: mem.Allocator) -> Error {
    if existing_ver, ok := chosen^[pkg_name]; ok {
        if !satisfies(existing_ver, req) {
            return .Conflict
        }
        return .None
    }
    pkg, ok := repo^[pkg_name]
    if !ok {
        return .PackageNotFound
    }
    compatible: [dynamic]string
    defer {
        for str in compatible {
            delete(str, allocator)
        }
        delete(compatible)
    }
    for v in pkg.versions {
        if satisfies(v.version, req) {
            append(&compatible, strings.clone(v.version, allocator))
        }
    }
    if len(compatible) == 0 {
        return .VersionNotFound
    }
    sort.sort(sort.Interface{
        collection = &compatible,
        len = proc(it: sort.Interface) -> int { return len((^[dynamic]string)(it.collection)^) },
              less = proc(it: sort.Interface, i, j: int) -> bool {
                  arr := (^[dynamic]string)(it.collection)^
                  return compare_versions(arr[i], arr[j]) < 0
              },
              swap = proc(it: sort.Interface, i, j: int) {
                  arr := (^[dynamic]string)(it.collection)^
                  arr[i], arr[j] = arr[j], arr[i]
              },
    })
    chosen^[pkg_name] = compatible[len(compatible)-1]
    return .None
}
// Stack frame used in iterative DFS
_StackFrame :: struct {
    pkg: string,
    req: string,
    deps: map[string]string,
    index: int,
}
resolve_deps_iterative :: proc(allocator: mem.Allocator, repo: ^Repo, root_pkg: string, root_req: string, chosen: ^map[string]string, order: ^[dynamic]struct {pkg: string, ver: string}) -> Error {
    stack: [dynamic]_StackFrame
    defer {
        for frame in stack {
            for dk, dv in frame.deps {
                delete(dk, allocator)
                delete(dv, allocator)
            }
            delete(frame.deps)
        }
        delete(stack)
    }
    visiting: map[string]bool
    defer {
        for k in visiting {
            delete(k, allocator)
        }
        delete(visiting)
    }
    append(&stack, _StackFrame{root_pkg, root_req, {}, 0})
    for len(stack) > 0 {
        top := &stack[len(stack)-1]
        if top.index == 0 {
            if visiting[top.pkg] {
                return .Cycle
            }
            visiting[top.pkg] = true
            err := choose_version(repo, top.pkg, top.req, chosen, allocator)
            if err != .None {
                return err
            }
            ver := chosen^[top.pkg]
            pkg := repo^[top.pkg]
            found := false
            for v in pkg.versions {
                if v.version == ver {
                    top.deps = v.deps
                    found = true
                    break
                }
            }
            if !found {
                return .VersionNotFound
            }
        }
        // Collect dep keys for current index
        deps_keys: [dynamic]string
        defer delete(deps_keys)
        for dep in top.deps {
            append(&deps_keys, dep)
        }
        if top.index < len(deps_keys) {
            dep := deps_keys[top.index]
            dep_req := top.deps[dep]
            top.index += 1
            append(&stack, _StackFrame{dep, dep_req, {}, 0})
        } else {
            pkg_name := top.pkg
            delete_key(&visiting, pkg_name)
            append(order, struct{pkg: string, ver: string}{pkg_name, chosen^[pkg_name]})
            pop(&stack)
        }
    }
    return .None
}
