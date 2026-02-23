package hpm

import "core:fmt"
import "core:os"
import "core:mem"
import "core:strings"
import "core:time"
import "core:sort"
import "core:path/filepath"
import "core:sys/linux"
import "core:encoding/json"
import "core:crypto/sha2"
import "core:thread"
import "core:io"

VERSION :: "0.6"
STORE_PATH :: "/usr/lib/HackerOS/hpm/store/"
BACKEND_PATH :: "/usr/lib/HackerOS/hpm/backend"
REPO_JSON_URL :: "https://raw.githubusercontent.com/HackerOS-Linux-System/Hacker-Package-Manager/main/repo/repo.json"
VERSION_URL :: "https://raw.githubusercontent.com/HackerOS-Linux-System/Hacker-Package-Manager/main/version.hacker"
LOCAL_VERSION_FILE :: "/usr/lib/HackerOS/hpm/version.json"
RELEASES_BASE :: "https://github.com/HackerOS-Linux-System/Hacker-Package-Manager/releases/download/v"
STATE_PATH :: "/var/lib/hpm/state.json"
STATE_TMP_PATH :: "/var/lib/hpm/state.json.tmp"
LOCK_PATH :: "/var/lib/hpm/lock"
LOG_PATH :: "/var/log/hpm.log"
CACHE_PATH :: "/var/cache/hpm/"
// Kolory ANSI
COLOR_GREEN :: "\033[1;32m"
COLOR_YELLOW :: "\033[1;33m"
COLOR_RED :: "\033[1;31m"
COLOR_CYAN :: "\033[1;36m"
COLOR_MAGENTA :: "\033[1;35m"
COLOR_BLUE :: "\033[1;34m"
COLOR_RESET :: "\033[0m"

Error :: enum {
    None,
    InvalidArgs,
    RepoLoadFailed,
    StateLoadFailed,
    LockFailed,
    DownloadFailed,
    ChecksumMismatch,
    UnpackFailed,
    BackendFailed,
    VersionNotFound,
    PackageNotFound,
    Conflict,
    Cycle,
    SymlinkFailed,
    ChmodFailed,
    RemoveFailed,
    SwitchFailed,
    UpgradeFailed,
    SearchFailed,
    InfoFailed,
    ListFailed,
    CleanFailed,
    VerifyFailed,
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
    err: Error
    switch command {
        case "refresh":
            err = refresh(allocator)
        case "install":
            if len(args) < 2 {
                err = .InvalidArgs
            } else {
                err = install(allocator, args[1:])
            }
        case "remove":
            if len(args) < 2 {
                err = .InvalidArgs
            } else {
                err = remove(allocator, args[1])
            }
        case "update":
            err = update(allocator)
        case "switch":
            if len(args) < 3 {
                err = .InvalidArgs
            } else {
                err = switch_version(allocator, args[1], args[2])
            }
        case "upgrade":
            err = upgrade(allocator)
        case "run":
            if len(args) < 2 {
                err = .InvalidArgs
            } else {
                run_code := run_tool(allocator, args[1:])
                os.exit(run_code)
            }
        case "build":
            if len(args) < 2 {
                err = .InvalidArgs
            } else {
                err = build(allocator, args[1])
            }
        case "search":
            if len(args) < 2 {
                err = .InvalidArgs
            } else {
                err = search(allocator, strings.join(args[1:], " "))
            }
        case "info":
            if len(args) < 2 {
                err = .InvalidArgs
            } else {
                err = info(allocator, args[1])
            }
        case "list":
            err = list_installed(allocator)
        case "clean":
            err = clean_cache(allocator)
        case "pin":
            if len(args) < 3 {
                err = .InvalidArgs
            } else {
                err = pin(allocator, args[1], args[2])
            }
        case "unpin":
            if len(args) < 2 {
                err = .InvalidArgs
            } else {
                err = unpin(allocator, args[1])
            }
        case "outdated":
            err = outdated(allocator)
        case "verify":
            if len(args) < 2 {
                err = .InvalidArgs
            } else {
                err = verify(allocator, args[1])
            }
        case "deps":
            if len(args) < 2 {
                err = .InvalidArgs
            } else {
                err = deps(allocator, args[1])
            }
        case:
            print_help()
            return
    }
    if err != .None {
        print_error(err)
        os.exit(1)
    }
}

print_help :: proc() {
    fmt.printf("%sHPM %s - Hacker Package Manager%s\n", COLOR_GREEN, VERSION, COLOR_RESET)
    fmt.println("Usage: hpm <command> [args]")
    fmt.println("Commands:")
    fmt.printf(" %srefresh%s Refresh package index\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %sinstall%s <pkg>[@ver] Install package (with optional version)\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %sremove%s <pkg>[@ver] Remove package (with optional version)\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %supdate%s Update all installed packages\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %sswitch%s <pkg> <ver> Switch to specific version\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %supgrade%s Upgrade HPM itself\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %srun%s <pkg>[@ver] <bin> Run tool from package\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %sbuild%s <name> Build .hpm package from current directory\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %ssearch%s <query> Search packages by name/description\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %sinfo%s <pkg> Show package info\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %slist%s List installed packages\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %sclean%s Clean cache\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %spin%s <pkg> <ver> Pin package to version\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %sunpin%s <pkg> Unpin package\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %soutdated%s List outdated packages\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %sverify%s <pkg> Verify package checksum\n", COLOR_CYAN, COLOR_RESET)
    fmt.printf(" %sdeps%s <pkg> Show dependency tree\n", COLOR_CYAN, COLOR_RESET)
}

print_error :: proc(err: Error) {
    fmt.printf("%s✖ Error: ", COLOR_RED)
    switch err {
        case .None: // Should not happen
        case .InvalidArgs:
            fmt.printf("Invalid arguments provided.%s\n", COLOR_RESET)
        case .RepoLoadFailed:
            fmt.printf("Failed to load repository. Try 'hpm refresh'.%s\n", COLOR_RESET)
        case .StateLoadFailed:
            fmt.printf("Failed to load state.%s\n", COLOR_RESET)
        case .LockFailed:
            fmt.printf("Lock file exists. Another operation in progress.%s\n", COLOR_RESET)
        case .DownloadFailed:
            fmt.printf("Download failed.%s\n", COLOR_RESET)
        case .ChecksumMismatch:
            fmt.printf("Checksum mismatch.%s\n", COLOR_RESET)
        case .UnpackFailed:
            fmt.printf("Unpack failed.%s\n", COLOR_RESET)
        case .BackendFailed:
            fmt.printf("Backend operation failed.%s\n", COLOR_RESET)
        case .VersionNotFound:
            fmt.printf("Version not found.%s\n", COLOR_RESET)
        case .PackageNotFound:
            fmt.printf("Package not found.%s\n", COLOR_RESET)
        case .Conflict:
            fmt.printf("Dependency conflict.%s\n", COLOR_RESET)
        case .Cycle:
            fmt.printf("Dependency cycle detected.%s\n", COLOR_RESET)
        case .SymlinkFailed:
            fmt.printf("Failed to create symlink.%s\n", COLOR_RESET)
        case .ChmodFailed:
            fmt.printf("Failed to set permissions.%s\n", COLOR_RESET)
        case .RemoveFailed:
            fmt.printf("Remove failed.%s\n", COLOR_RESET)
        case .SwitchFailed:
            fmt.printf("Switch failed.%s\n", COLOR_RESET)
        case .UpgradeFailed:
            fmt.printf("Upgrade failed.%s\n", COLOR_RESET)
        case .SearchFailed:
            fmt.printf("Search failed.%s\n", COLOR_RESET)
        case .InfoFailed:
            fmt.printf("Info failed.%s\n", COLOR_RESET)
        case .ListFailed:
            fmt.printf("List failed.%s\n", COLOR_RESET)
        case .CleanFailed:
            fmt.printf("Clean failed.%s\n", COLOR_RESET)
        case .VerifyFailed:
            fmt.printf("Verify failed.%s\n", COLOR_RESET)
    }
}
