const std = @import("std");
const cli_pr = @import("cli/pr.zig");

pub const Config = struct {
    token: []const u8,
    repo: []const u8,
    host: []const u8,
};

const SubCommand = struct {
    name: []const u8,
    run: *const fn (std.mem.Allocator, Config, []const []const u8) anyerror!void,
};

// Add new subcommands here
const dispatch_table = [_]SubCommand{
    .{ .name = "pr", .run = cli_pr.run },
};

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const alloc = gpa.allocator();

    const args = try std.process.argsAlloc(alloc);
    defer std.process.argsFree(alloc, args);

    if (args.len < 2) {
        printUsage();
        std.process.exit(1);
    }

    const subcmd = args[1];

    if (std.mem.eql(u8, subcmd, "version") or std.mem.eql(u8, subcmd, "--version")) {
        try std.fs.File.stdout().deprecatedWriter().print("gh-lint v0.1.0\n", .{});
        return;
    }

    if (std.mem.eql(u8, subcmd, "help") or std.mem.eql(u8, subcmd, "--help")) {
        printUsage();
        return;
    }

    const cfg = try loadConfig(alloc);

    for (dispatch_table) |cmd| {
        if (std.mem.eql(u8, cmd.name, subcmd)) {
            try cmd.run(alloc, cfg, args[2..]);
            return;
        }
    }

    try std.fs.File.stderr().deprecatedWriter().print("unknown subcommand: {s}\n", .{subcmd});
    printUsage();
    std.process.exit(1);
}

fn validateHost(host: []const u8) !void {
    if (host.len == 0) return error.InvalidHost;
    // Block localhost
    if (std.mem.startsWith(u8, host, "localhost")) return error.InvalidHost;
    // Block IP addresses (starts with digit)
    if (host[0] >= '0' and host[0] <= '9') return error.InvalidHost;
    // Must contain a dot (valid hostname)
    if (std.mem.indexOf(u8, host, ".") == null) return error.InvalidHost;
}

fn loadConfig(alloc: std.mem.Allocator) !Config {
    const token = try resolveToken(alloc);
    const repo = std.process.getEnvVarOwned(alloc, "GH_REPO") catch try alloc.dupe(u8, "");
    const host = std.process.getEnvVarOwned(alloc, "GH_HOST") catch try alloc.dupe(u8, "api.github.com");
    try validateHost(host);
    return Config{ .token = token, .repo = repo, .host = host };
}

fn resolveToken(alloc: std.mem.Allocator) ![]const u8 {
    if (std.process.getEnvVarOwned(alloc, "GH_TOKEN")) |token| {
        return token;
    } else |_| {}
    if (std.process.getEnvVarOwned(alloc, "GH_ENTERPRISE_TOKEN")) |token| {
        return token;
    } else |_| {}
    // Fallback: run `gh auth token`
    var child = std.process.Child.init(&[_][]const u8{ "gh", "auth", "token" }, alloc);
    child.stdout_behavior = .Pipe;
    child.stderr_behavior = .Ignore;
    try child.spawn();
    var out_buf: [4096]u8 = undefined;
    const n = try child.stdout.?.readAll(&out_buf);
    _ = try child.wait();
    const out = try alloc.dupe(u8, std.mem.trim(u8, out_buf[0..n], "\n\r "));
    @memset(&out_buf, 0);
    return out;
}

fn printUsage() void {
    const usage =
        \\Usage: gh lint <subcommand> [flags]
        \\
        \\Subcommands:
        \\  pr <PR_NUMBER>   Lint a pull request
        \\  version          Show version
        \\  help             Show this help
        \\
    ;
    std.fs.File.stdout().deprecatedWriter().print("{s}", .{usage}) catch {};
}
