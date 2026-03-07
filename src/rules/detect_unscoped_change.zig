const std = @import("std");
const rule = @import("rule.zig");
const diff_parser = @import("../util/diff_parser.zig");
const file_classifier = @import("../util/file_classifier.zig");

const NOISE_THRESHOLD = 5; // Domains with changes at or below this line count are ignored

pub fn run(alloc: std.mem.Allocator, ctx: rule.RuleContext) ![]rule.RuleResult {
    // Aggregate changed lines and file paths per domain
    const DomainData = struct {
        lines: u32,
        files: std.ArrayList([]const u8),
    };

    var domain_map = std.EnumArray(diff_parser.Domain, DomainData).initUndefined();
    for (std.enums.values(diff_parser.Domain)) |d| {
        domain_map.set(d, DomainData{
            .lines = 0,
            .files = .empty,
        });
    }

    for (ctx.pr_files) |f| {
        const domain = file_classifier.classifyFile(f);
        var data = domain_map.getPtr(domain);
        data.lines += f.additions + f.deletions;
        try data.files.append(alloc, f.filename);
    }

    // Noise filtering: count domains with lines > NOISE_THRESHOLD, excluding test/unknown
    var active_domains: std.ArrayList(diff_parser.Domain) = .empty;
    for (std.enums.values(diff_parser.Domain)) |d| {
        if (d == .@"test" or d == .unknown) continue;
        const data = domain_map.get(d);
        if (data.lines > NOISE_THRESHOLD) {
            try active_domains.append(alloc, d);
        }
    }

    const domain_count = active_domains.items.len;

    // PASS determination
    const is_pass = blk: {
        if (domain_count <= 1) break :blk true;
        if (domain_count == 2) {
            // docs + one other domain is acceptable
            for (active_domains.items) |d| {
                if (d == .docs) break :blk true;
            }
        }
        break :blk false;
    };

    if (is_pass) {
        const results = try alloc.alloc(rule.RuleResult, 1);
        results[0] = rule.RuleResult{
            .rule_id = "detect-unscoped-change",
            .severity = .pass,
            .message = "PR is well-scoped",
            .affected_files = &[_][]const u8{},
            .suggestion = null,
        };
        return results;
    }

    // Warning/error determination
    const severity: rule.Severity = if (domain_count >= 3) .@"error" else .warning;

    // Build affected file list and detail message
    var affected: std.ArrayList([]const u8) = .empty;
    var detail_buf: std.ArrayList(u8) = .empty;
    const writer = detail_buf.writer(alloc);

    for (active_domains.items) |d| {
        const data = domain_map.get(d);
        try writer.print("  {s} ({d} lines):", .{ d.name(), data.lines });
        for (data.files.items) |f| {
            try writer.print(" {s}", .{f});
            try affected.append(alloc, f);
        }
        try writer.print("\n", .{});
    }

    const message = try std.fmt.allocPrint(
        alloc,
        "PR touches {d} unrelated domains",
        .{domain_count},
    );

    const results = try alloc.alloc(rule.RuleResult, 1);
    results[0] = rule.RuleResult{
        .rule_id = "detect-unscoped-change",
        .severity = severity,
        .message = message,
        .affected_files = try affected.toOwnedSlice(alloc),
        .suggestion = try detail_buf.toOwnedSlice(alloc),
    };
    return results;
}
