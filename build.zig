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

    // tree-sitter core (brew)
    exe.addIncludePath(.{ .cwd_relative = "/opt/homebrew/Cellar/tree-sitter/0.26.6/include" });
    exe.addLibraryPath(.{ .cwd_relative = "/opt/homebrew/Cellar/tree-sitter/0.26.6/lib" });
    exe.linkSystemLibrary("tree-sitter");

    // tree-sitter-go grammar (brew)
    exe.addIncludePath(.{ .cwd_relative = "/opt/homebrew/Cellar/tree-sitter-go/0.25.0/include" });
    exe.addLibraryPath(.{ .cwd_relative = "/opt/homebrew/Cellar/tree-sitter-go/0.25.0/lib" });
    exe.linkSystemLibrary("tree-sitter-go");

    // tree-sitter-python grammar (brew)
    exe.addIncludePath(.{ .cwd_relative = "/opt/homebrew/Cellar/tree-sitter-python/0.25.0/include" });
    exe.addLibraryPath(.{ .cwd_relative = "/opt/homebrew/Cellar/tree-sitter-python/0.25.0/lib" });
    exe.linkSystemLibrary("tree-sitter-python");

    // tree-sitter-typescript grammar (vendored C source)
    exe.addIncludePath(.{ .cwd_relative = "deps/tree-sitter-typescript" });
    exe.addCSourceFile(.{
        .file = b.path("deps/tree-sitter-typescript/parser.c"),
        .flags = &.{"-std=c11"},
    });
    exe.addCSourceFile(.{
        .file = b.path("deps/tree-sitter-typescript/scanner.c"),
        .flags = &.{"-std=c11"},
    });

    exe.linkLibC();
    b.installArtifact(exe);
}
