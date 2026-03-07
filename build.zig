const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const exe = b.addExecutable(.{
        .name = "gh-lint",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });

    const ts = b.lazyDependency("tree_sitter", .{});
    const ts_go = b.lazyDependency("tree_sitter_go", .{});
    const ts_py = b.lazyDependency("tree_sitter_python", .{});
    const ts_ts = b.lazyDependency("tree_sitter_typescript", .{});

    // tree-sitter core
    if (ts) |dep| {
        exe.addIncludePath(dep.path("lib/include"));
        exe.addIncludePath(dep.path("lib/src"));
        exe.addCSourceFile(.{
            .file = dep.path("lib/src/lib.c"),
            .flags = &.{"-std=c11"},
        });
    }

    // tree-sitter-go
    if (ts_go) |dep| {
        exe.addIncludePath(dep.path("bindings/c"));
        exe.addIncludePath(dep.path("src"));
        exe.addCSourceFile(.{
            .file = dep.path("src/parser.c"),
            .flags = &.{"-std=c11"},
        });
    }

    // tree-sitter-python
    if (ts_py) |dep| {
        exe.addIncludePath(dep.path("bindings/c"));
        exe.addIncludePath(dep.path("src"));
        exe.addCSourceFile(.{
            .file = dep.path("src/parser.c"),
            .flags = &.{"-std=c11"},
        });
        exe.addCSourceFile(.{
            .file = dep.path("src/scanner.c"),
            .flags = &.{"-std=c11"},
        });
    }

    // tree-sitter-typescript
    // Header is at bindings/c/tree-sitter-typescript.h (no tree_sitter/ prefix),
    // but Zig source expects tree_sitter/tree-sitter-typescript.h.
    // Use WriteFiles to create a virtual include directory with the correct path.
    if (ts_ts) |dep| {
        const wf = b.addWriteFiles();
        _ = wf.addCopyFile(dep.path("bindings/c/tree-sitter-typescript.h"), "tree_sitter/tree-sitter-typescript.h");
        exe.addIncludePath(wf.getDirectory());
        exe.addIncludePath(dep.path("typescript/src"));
        exe.addCSourceFile(.{
            .file = dep.path("typescript/src/parser.c"),
            .flags = &.{"-std=c11"},
        });
        exe.addCSourceFile(.{
            .file = dep.path("typescript/src/scanner.c"),
            .flags = &.{"-std=c11"},
        });
    }

    exe.linkLibC();
    b.installArtifact(exe);
}
