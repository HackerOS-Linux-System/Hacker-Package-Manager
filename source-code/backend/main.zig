const std = @import("std");
const os = std.os;
const fs = std.fs;
const process = std.process;

pub fn main() !void {
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();
    const allocator = arena.allocator();

    const args = try process.argsAlloc(allocator);
    defer process.argsFree(allocator, args);

    if (args.len < 3) {
        std.debug.print("Usage: backend [install|remove] <package> <path>\n", .{});
        return;
    }

    const command = args[1];
    const package = args[2];
    const path = args[3];

    if (std.mem.eql(u8, command, "install")) {
        try install(package, path);
    } else if (std.mem.eql(u8, command, "remove")) {
        try remove(package, path);
    } else {
        std.debug.print("Unknown command: {s}\n", .{command});
    }
}

fn install(package: []const u8, path: []const u8) !void {
    std.debug.print("Installing {s} with isolation at {s}\n", .{package, path});
    // Simulate bubblewrap isolation: Use bwrap if available
    // For simplicity, assume bwrap is installed and use it to sandbox
    const bwrap_args = [_][]const u8{
        "bwrap",
        "--ro-bind", "/usr", "/usr",
        "--ro-bind", "/lib", "/lib",
        "--ro-bind", "/lib64", "/lib64",
        "--ro-bind", "/bin", "/bin",
        "--ro-bind", "/etc", "/etc",
        "--bind", path, "/app",
        "--chdir", "/app",
        "--unshare-all",
        "--share-net", // If network needed
        "--", "sh", "-c", "echo Isolated install complete" // Replace with actual install script
    };
    var child = std.ChildProcess.init(&bwrap_args, std.heap.page_allocator);
    try child.spawn();
    const term = try child.wait();
    _ = term; // Check exit code if needed
    std.debug.print("Installation complete for {s}\n", .{package});
}

fn remove(package: []const u8, path: []const u8) !void {
    std.debug.print("Removing {s} from {s}\n", .{package, path});
    // Cleanup isolation (simplified)
    try fs.cwd().deleteTree(path);
    std.debug.print("Removal complete for {s}\n", .{package});
}
