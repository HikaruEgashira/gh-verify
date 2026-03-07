const std = @import("std");
const Config = @import("../main.zig").Config;
const pr_api = @import("../github/pr_api.zig");
const engine = @import("../rules/engine.zig");
const rule = @import("../rules/rule.zig");
const formatter = @import("../output/formatter.zig");

/// Entry point for the `gh lint pr` subcommand.
/// Contains no rule logic; delegates to each layer.
pub fn run(alloc: std.mem.Allocator, cfg: Config, args: []const []const u8) !void {
    const stderr = std.fs.File.stderr().deprecatedWriter();

    if (args.len == 0) {
        try stderr.print("Usage: gh lint pr <PR_NUMBER> [--repo OWNER/REPO] [--format human|json]\n", .{});
        std.process.exit(1);
    }

    if (std.mem.eql(u8, args[0], "list-rules")) {
        const stdout = std.fs.File.stdout().deprecatedWriter();
        try stdout.print("Available rules:\n", .{});
        for (engine.listRuleIds()) |id| {
            try stdout.print("  {s}\n", .{id});
        }
        return;
    }

    const pr_number = std.fmt.parseInt(u32, args[0], 10) catch {
        try stderr.print("Invalid PR number: {s}\n", .{args[0]});
        std.process.exit(1);
    };

    // Parse flags
    var format: formatter.Format = .human;
    var repo_override: ?[]const u8 = null;
    var i: usize = 1;
    while (i < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], "--format") and i + 1 < args.len) {
            i += 1;
            format = formatter.parseFormat(args[i]) catch {
                try stderr.print("Invalid format: {s} (use 'human' or 'json')\n", .{args[i]});
                std.process.exit(1);
            };
        } else if (std.mem.eql(u8, args[i], "--repo") and i + 1 < args.len) {
            i += 1;
            repo_override = args[i];
        }
    }

    // Resolve OWNER/REPO
    const repo_str = repo_override orelse cfg.repo;
    const slash_idx = std.mem.indexOf(u8, repo_str, "/") orelse {
        try stderr.print("Could not resolve repo. Use --repo OWNER/REPO or set GH_REPO env var.\n", .{});
        std.process.exit(1);
    };
    const owner = repo_str[0..slash_idx];
    const repo_name = repo_str[slash_idx + 1 ..];

    // Fetch data from GitHub API
    const pr_files = pr_api.getPrFiles(alloc, cfg, owner, repo_name, pr_number) catch |err| {
        try stderr.print("Failed to fetch PR files: {}\n", .{err});
        std.process.exit(1);
    };

    const pr_meta = pr_api.getPrMetadata(alloc, cfg, owner, repo_name, pr_number) catch |err| {
        try stderr.print("Failed to fetch PR metadata: {}\n", .{err});
        std.process.exit(1);
    };

    // Run rules
    const ctx = rule.RuleContext{
        .pr_files = pr_files,
        .pr_metadata = pr_meta,
    };
    const results = try engine.runAll(alloc, ctx);

    // Output
    try formatter.print(alloc, format, results);

    // Exit with code 1 if any errors
    for (results) |r| {
        if (r.severity == .@"error") std.process.exit(1);
    }
}
