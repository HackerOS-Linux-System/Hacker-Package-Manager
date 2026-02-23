package hpm
import "core:fmt"
import "core:os"
import "core:strings"
import "core:strconv"
import "core:crypto/sha2"
import "core:sys/linux"
import "core:time"
import "core:thread"
import "core:mem"
import "core:path/filepath"
import "core:io"
WIFEXITED :: proc "contextless" (status: i32) -> bool { return ((status) & 0o177) == 0 }
WEXITSTATUS :: proc "contextless" (status: i32) -> i32 { return ((status) >> 8) & 0x000000ff }
run_command :: proc(args: []string) -> (int, Error) {
    if len(args) == 0 {
        return 1, .InvalidArgs
    }
    exec_path: string
    if filepath.is_abs(args[0]) || strings.contains_rune(args[0], '/') {
        exec_path = args[0]
    } else {
        path_env := os.get_env("PATH")
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
            return 127, .None // Command not found
        }
    }
    args_c: [dynamic]cstring
    defer delete(args_c)
    for arg in args {
        append(&args_c, strings.clone_to_cstring(arg, context.temp_allocator))
    }
    append(&args_c, nil)
    exec_path_c := strings.clone_to_cstring(exec_path, context.temp_allocator)
    pid, ferr := linux.fork()
    if ferr != .NONE {
        return 1, .BackendFailed
    }
    if pid == 0 {
        // Child
        linux.execve(exec_path_c, raw_data(args_c), nil)
        linux.exit(1)
    } else if pid > 0 {
        // Parent
        status: u32
        _, werr := linux.waitpid(pid, &status, {}, nil)
        if werr != .NONE {
            return 1, .BackendFailed
        }
        if WIFEXITED(i32(status)) {
            return int(WEXITSTATUS(i32(status))), .None
        } else {
            return 1, .BackendFailed
        }
    }
    return 1, .BackendFailed
}
download_file :: proc(allocator: mem.Allocator, url: string, path: string) -> Error {
    fmt.printf("%s↓ Downloading %s...%s\n", COLOR_YELLOW, url, COLOR_RESET)
    args := []string{"curl", "-L", "--progress-bar", "-o", path, url}
    code, err := run_command(args[:])
    if code != 0 || err != .None {
        return .DownloadFailed
    }
    fmt.printf("%s✔ Download complete.%s\n", COLOR_GREEN, COLOR_RESET)
    return .None
}
compute_sha256_stream :: proc(allocator: mem.Allocator, path: string) -> (string, Error) {
    f, err := os.open(path, os.O_RDONLY, 0)
    if err != os.ERROR_NONE {
        return "", .ChecksumMismatch
    }
    defer os.close(f)
    ctx: sha2.Context_256
    sha2.init_256(&ctx)
    buf: [4096]u8
    for {
        n, rerr := os.read(f, buf[:])
        if rerr != os.ERROR_NONE || n == 0 {
            break
        }
        sha2.update(&ctx, buf[:n])
    }
    hash: [sha2.DIGEST_SIZE_256]u8
    sha2.final(&ctx, hash[:])
    sb: strings.Builder
    strings.builder_init(&sb, allocator)
    for b in hash {
        fmt.sbprintf(&sb, "%02x", b)
    }
    return strings.to_string(sb), .None
}
readlink :: proc(path: string, allocator: mem.Allocator) -> (string, bool) {
    MAX_PATH :: 4096
    buf := make([]u8, MAX_PATH, allocator)
    n, err := linux.readlink(strings.clone_to_cstring(path, context.temp_allocator), buf[:])
    if err != .NONE || n < 0 {
        delete(buf)
        return "", false
    }
    res := string(buf[:n])
    delete(buf)
    return res, true
}
log_to_file :: proc(level: string, message: string) {
    timestamp := time.now()
    ts_str := fmt.tprintf("%v", timestamp)
    entry := fmt.tprintf("%s [%s] %s\n", ts_str, level, message)
    f, err := os.open(LOG_PATH, os.O_APPEND | os.O_CREATE | os.O_WRONLY, 0o644)
    if err != os.ERROR_NONE {
        return
    }
    defer os.close(f)
    os.write_string(f, entry)
}
acquire_lock :: proc() -> Error {
    if os.exists(LOCK_PATH) {
        data, ok := os.read_entire_file(LOCK_PATH, context.temp_allocator)
        if ok {
            pid_str := string(data)
            pid, pok := strconv.parse_int(pid_str)
            if pok {
                if linux.kill(linux.Pid(i32(pid)), linux.Signal(0)) == .NONE {
                    print_error(.LockFailed)
                    return .LockFailed
                } else {
                    os.remove(LOCK_PATH)
                }
            }
        }
    }
    my_pid := int(linux.getpid())
    os.write_entire_file(LOCK_PATH, transmute([]u8)fmt.tprintf("%d", my_pid))
    return .None
}
release_lock :: proc() {
    os.remove(LOCK_PATH)
}
spinner_thread :: proc(t: ^thread.Thread) {
    done := (^bool)(t.data)
    spinner := [?]string{"⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"}
    i := 0
    for !done^ {
        fmt.printf("\r%s", spinner[i])
        i = (i + 1) % len(spinner)
        time.sleep(100 * time.Millisecond)
    }
    fmt.printf("\r ")
}
start_spinner :: proc() -> (^bool, ^thread.Thread) {
    done := new(bool)
    done^ = false
    t := thread.create(spinner_thread)
    t.data = rawptr(done)
    thread.start(t)
    return done, t
}
stop_spinner :: proc(done: ^bool, t: ^thread.Thread) {
    done^ = true
    thread.join(t)
    free(done)
    thread.destroy(t)
}
