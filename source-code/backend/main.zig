const std = @import("std");
const os = std.os;
const fs = std.fs;
const process = std.process;
const json = std.json;
const mem = std.mem;
const Allocator = std.mem.Allocator;
const Manifest = struct {
    name: []const u8,
    version: []const u8,
    deps: ?[]const []const u8 = null,
    bins: ?[]const []const u8 = null,
    sandbox: struct {
        network: bool = false,
        filesystem: ?[]const []const u8 = null,
    } = .{},
    pub fn deinit(self: Manifest, allocator: Allocator) void {
        allocator.free(self.name);
        allocator.free(self.version);
        if (self.deps) |deps| {
            for (deps) |dep| allocator.free(dep);
            allocator.free(deps);
        }
        if (self.bins) |bins| {
            for (bins) |bin| allocator.free(bin);
            allocator.free(bins);
        }
        if (self.sandbox.filesystem) |fs_paths| {
            for (fs_paths) |path| allocator.free(path);
            allocator.free(fs_paths);
        }
    }
};
const PackageInfo = struct {
    version: []const u8,
    checksum: []const u8,
};
const State = struct {
    packages: std.StringHashMap(PackageInfo),
    pub fn deinit(self: *State, allocator: Allocator) void {
        var it = self.packages.iterator();
        while (it.next()) |entry| {
            allocator.free(entry.key_ptr.*);
            allocator.free(entry.value_ptr.version);
            allocator.free(entry.value_ptr.checksum);
        }
        self.packages.deinit();
    }
};
const ErrorCode = enum(i32) {
    Success = 0,
    InvalidArgs = 1,
    PackageNotFound = 2,
    DependencyCycle = 3,
    InstallFailed = 4,
    RemoveFailed = 5,
    VerificationFailed = 6,
    UnknownCommand = 99,
};
pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();
    const args = try process.argsAlloc(allocator);
    defer process.argsFree(allocator, args);
    if (args.len < 3) {
        try outputError(allocator, .InvalidArgs, "Usage: backend [install|remove|verify] <package> <path> [checksum]");
        return;
    }
    const command = args[1];
    const package = args[2];
    const path = args[3];
    const checksum = if (args.len > 4) args[4] else null;
    if (mem.eql(u8, command, "install")) {
        try install(allocator, package, path, checksum);
    } else if (mem.eql(u8, command, "remove")) {
        try remove(allocator, package, path);
    } else if (mem.eql(u8, command, "verify")) {
        const chk = checksum orelse {
            try outputError(allocator, .InvalidArgs, "Checksum required for verify");
            return;
        };
        try verify(allocator, path, chk);
        const payload = .{ .success = true };
        const output = try stringifyAlloc(allocator, payload, .{});
        defer allocator.free(output);
        try std.io.getStdOut().writeAll(output);
    } else {
        try outputError(allocator, .UnknownCommand, "Unknown command");
    }
}
fn stringifyAlloc(allocator: Allocator, value: anytype, options: anytype) ![]u8 {
    var buf = std.ArrayList(u8).init(allocator);
    errdefer buf.deinit();
    try json.stringify(value, options, buf.writer());
    return try buf.toOwnedSlice();
}
fn outputError(allocator: Allocator, code: ErrorCode, msg: []const u8) !void {
    const payload = .{ .err = .{ .code = @intFromEnum(code), .message = msg } };
    const output = try stringifyAlloc(allocator, payload, .{});
    defer allocator.free(output);
    try std.io.getStdErr().writeAll(output);
    std.process.exit(@intFromEnum(code));
}
fn loadManifest(allocator: Allocator, path: []const u8) !Manifest {
    const manifest_path = try std.fmt.allocPrint(allocator, "{s}/manifest.json", .{path});
    defer allocator.free(manifest_path);
    const file = try fs.cwd().openFile(manifest_path, .{});
    defer file.close();
    const content = try file.readToEndAlloc(allocator, 1024 * 1024);
    defer allocator.free(content);
    const parsed = try json.parseFromSlice(Manifest, allocator, content, .{ .allocate = .alloc_always });
    return parsed.value;
}
fn install(allocator: Allocator, package: []const u8, path: []const u8, checksum: ?[]const u8) !void {
    const tmp_path = try std.fmt.allocPrint(allocator, "{s}.tmp", .{path});
    defer allocator.free(tmp_path);
    fs.cwd().makeDir(tmp_path) catch |err| if (err != error.PathAlreadyExists) return err;
    // Simulate install in tmp
    // In real: extract to tmp, but since CLI does extract, assume it's already in tmp_path
    // Load manifest
    const manifest = try loadManifest(allocator, tmp_path);
    defer manifest.deinit(allocator);
    // Handle deps recursively
    if (manifest.deps) |deps| {
        for (deps) |dep| {
            // TODO: Full DAG, cycle detection
            // For now: recursive install (assume no cycles)
            // In CLI: handle this, but for backend, assume deps installed
            std.debug.print("Installing dep: {s}\n", .{dep});
        }
    }
    // Setup sandbox and run install script if any
    try setupSandbox(allocator, package, tmp_path, &manifest);
    // Verify checksum if provided (archive checksum, assume file in path)
    if (checksum) |chk| {
        try verify(allocator, tmp_path, chk);
    }
    // On success, rename
    try fs.cwd().rename(tmp_path, path);
    // Update state
    try updateState(allocator, package, manifest.version, checksum orelse "none");
    const payload = .{ .success = true, .package = package };
    const output = try stringifyAlloc(allocator, payload, .{});
    defer allocator.free(output);
    try std.io.getStdOut().writeAll(output);
}
fn remove(allocator: Allocator, package: []const u8, path: []const u8) !void {
    // Load manifest for cleanup
    const manifest = try loadManifest(allocator, path);
    defer manifest.deinit(allocator);
    // Cleanup bins, etc.
    if (manifest.bins) |bins| {
        for (bins) |bin| {
            const bin_path = try std.fmt.allocPrint(allocator, "/usr/bin/{s}", .{bin});
            defer allocator.free(bin_path);
            fs.cwd().deleteFile(bin_path) catch {};
        }
    }
    try fs.cwd().deleteTree(path);
    // Remove from state
    var state = try loadState(allocator);
    defer state.deinit(allocator);
    if (state.packages.fetchRemove(package)) |kv| {
        allocator.free(kv.key);
        allocator.free(kv.value.version);
        allocator.free(kv.value.checksum);
    }
    try saveState(state);
    const payload = .{ .success = true, .package = package };
    const output = try stringifyAlloc(allocator, payload, .{});
    defer allocator.free(output);
    try std.io.getStdOut().writeAll(output);
}
fn verify(allocator: Allocator, path: []const u8, checksum: []const u8) !void {
    // Assume verifying archive or dir hash
    // For simplicity, hash a file in path, say manifest.json
    const manifest_path = try std.fmt.allocPrint(allocator, "{s}/manifest.json", .{path});
    defer allocator.free(manifest_path);
    const file = try fs.cwd().openFile(manifest_path, .{});
    defer file.close();
    var hasher = std.crypto.hash.sha2.Sha256.init(.{});
    var buf: [4096]u8 = undefined;
    while (true) {
        const bytes_read = try file.read(&buf);
        if (bytes_read == 0) break;
        hasher.update(buf[0..bytes_read]);
    }
    var hash: [std.crypto.hash.sha2.Sha256.digest_length]u8 = undefined;
    hasher.final(&hash);
    const computed = try std.fmt.allocPrint(allocator, "{x}", .{std.fmt.fmtSliceHexLower(&hash)});
    defer allocator.free(computed);
    if (!mem.eql(u8, computed, checksum)) {
        try outputError(allocator, .VerificationFailed, "Checksum mismatch");
    }
}
fn setupSandbox(allocator: Allocator, package: []const u8, path: []const u8, manifest: *const Manifest) !void {
    _ = package; // unused
    var args_list = std.ArrayList([]const u8).init(allocator);
    defer args_list.deinit();
    try args_list.appendSlice(&[_][]const u8{
        "bwrap",
        "--ro-bind", "/usr", "/usr",
        "--ro-bind", "/lib", "/lib",
        "--ro-bind", "/lib64", "/lib64",
        "--ro-bind", "/bin", "/bin",
        "--ro-bind", "/etc", "/etc",
        "--bind", path, "/app",
        "--chdir", "/app",
        "--unshare-all",
    });
    if (!manifest.sandbox.network) {
        try args_list.append("--unshare-net");
    } else {
        try args_list.append("--share-net");
    }
    if (manifest.sandbox.filesystem) |fs_paths| {
        for (fs_paths) |fs_path| {
            try args_list.appendSlice(&[_][]const u8{"--bind", fs_path, fs_path});
        }
    }
    try args_list.appendSlice(&[_][]const u8{
        "--", "sh", "-c", "echo Isolated install complete" // Replace with actual
    });
    const child = try std.ChildProcess.run(.{
        .allocator = allocator,
        .argv = args_list.items,
    });
    switch (child.term) {
        .Exited => |code| if (code != 0) {
            try outputError(allocator, .InstallFailed, "Sandbox failed");
        },
        else => try outputError(allocator, .InstallFailed, "Sandbox failed"),
    }
}
fn cleanup(path: []const u8) !void {
    try fs.cwd().deleteTree(path);
}
const STATE_PATH = "/var/lib/hpm/state.json";
fn loadState(allocator: Allocator) !State {
    const file = fs.cwd().openFile(STATE_PATH, .{}) catch |err| {
        if (err == error.FileNotFound) {
            return State{ .packages = std.StringHashMap(PackageInfo).init(allocator) };
        }
        return err;
    };
    defer file.close();
    const content = try file.readToEndAlloc(allocator, 1024 * 1024);
    defer allocator.free(content);
    const parsed = try json.parseFromSlice(State, allocator, content, .{ .allocate = .alloc_always });
    return parsed.value;
}
fn saveState(state: State) !void {
    const file = try fs.cwd().createFile(STATE_PATH, .{});
    defer file.close();
    try json.stringify(state, .{}, file.writer());
}
fn updateState(allocator: Allocator, package: []const u8, version: []const u8, checksum: []const u8) !void {
    var state = try loadState(allocator);
    defer state.deinit(allocator);
    try state.packages.put(
        try allocator.dupe(u8, package),
        .{
            .version = try allocator.dupe(u8, version),
            .checksum = try allocator.dupe(u8, checksum),
        },
    );
    try saveState(state);
}
