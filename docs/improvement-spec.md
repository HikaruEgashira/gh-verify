# ghlint Improvement Spec

## Current State

ghlint v1 classifies files into domains using path-based heuristics in `src/util/diff_parser.zig`.
All 14 benchmark cases pass (100%), but several known false positives/negatives exist.

### Known Issues

| Issue | File | Classified As | Should Be | Root Cause |
|-------|------|--------------|-----------|------------|
| supabase#43343 | `stripe-sessions-contest.tsx` | auth | ui | `containsPath("session")` matches "sessions" in event name |
| supabase#43343 | `schemas.ts` | database | ui | `containsPath("schema")` matches Zod schema file |
| express#7057 | `package.json` | unknown | config | `.json` not in config extension list |
| prisma#29270 | `prisma/_schema.ts` (in tests/) | database | test | `containsPath("schema")` checked before test path `prisma/` is not a test segment |
| General | `*.go`, `*.py`, `*.rs` source files | unknown | (varies) | No heuristic for generic source files |

---

## Section 1: Quick Wins (Path/Name Heuristic Improvements)

### 1.1 Add `.json` to config extensions

**Problem:** `package.json`, `tsconfig.json`, `composer.json` etc. are config files but classified as `unknown`.

**Fix in `diff_parser.zig` line 95:**
```zig
// config
if (containsSegment(lower, "config") or
    std.mem.endsWith(u8, lower, ".toml") or
    std.mem.endsWith(u8, lower, ".yaml") or
    std.mem.endsWith(u8, lower, ".yml") or
    std.mem.endsWith(u8, lower, ".json") or
    std.mem.endsWith(u8, lower, ".env"))
    return .config;
```

**Benchmark impact:** `pass-007` (express#7057 package.json) would change from `unknown` to `config`. Since it's a single-file PR, result stays PASS but now the domain is correctly attributed. Mixed PRs combining package.json + API changes would now be caught.

**Risk:** `.json` is generic. Files like `data.json` or `fixtures.json` would also become config. This is acceptable since JSON files in a project root are almost always configuration.

### 1.2 Tighten `session` matching to segment-only

**Problem:** `containsPath("session")` matches any substring including `stripe-sessions-contest.tsx`.

**Fix in `diff_parser.zig` line 59:** Change `containsPath` to `containsSegment`:
```zig
// auth
if (containsSegment(lower, "auth") or
    containsPath(lower, "login") or
    containsPath(lower, "token") or
    containsSegment(lower, "session") or   // was: containsPath
    containsPath(lower, "oauth"))
    return .auth;
```

**Benchmark impact:** `error-002` (supabase#43343) - `stripe-sessions-contest.tsx` would no longer be classified as `auth`. The file would fall through to `ui` (`.tsx` extension). This removes one false domain, changing the PR from 3 domains to 2 (ui + database).

**Note:** `containsSegment` requires `session` to be a full path segment (bounded by `/` or `.`), so `session/foo.ts` and `auth-session.ts` would NOT match. If we want hyphenated names like `auth-session` to match, we need to keep `containsPath` but add negative patterns. The simpler approach (containsSegment) is recommended as a first step.

### 1.3 Tighten `schema` matching to segment-only

**Problem:** `containsPath("schema")` matches `schemas.ts` (Zod validation schemas), `_schema.ts` (Prisma test fixtures).

**Fix in `diff_parser.zig` line 67:** Change `containsPath` to `containsSegment`:
```zig
// database
if (containsSegment(lower, "db") or
    containsSegment(lower, "database") or
    containsSegment(lower, "migration") or
    containsSegment(lower, "schema") or    // was: containsPath
    std.mem.endsWith(u8, lower, ".sql"))
    return .database;
```

**Benchmark impact:** `error-002` (supabase#43343) - `schemas.ts` would no longer match database (since "schema" is a substring of "schemas", not a segment). Combined with fix 1.2, this PR would become single-domain (ui only) = PASS. However, the benchmark expects ERROR, so this case definition needs updating or the expected result needs revision.

`error-001` (prisma#29270) - `_schema.ts` files: "schema" is not a segment here (preceded by `_`). They would no longer match database. These files are under `tests/functional/` paths but `prisma` is not recognized as a test segment. The `_schema.ts` suffix doesn't match test patterns either. They would become `unknown`. This is arguably more correct since they ARE test fixtures.

**Risk:** Legitimate database schema directories named `schema/` would still match. Files named `schema.sql` match via `.sql` extension. The main loss is files like `db-schema.ts` where "schema" is hyphen-separated -- these would no longer match. Consider adding `std.mem.endsWith(u8, lower, "/schema.ts")` or similar if needed.

### 1.4 Tighten remaining `containsPath` calls

**Current loose matchers:**
- `containsPath(lower, "login")` - Could match `loginButton.tsx` (acceptable)
- `containsPath(lower, "token")` - Could match `tokenizer.ts` (false positive)
- `containsPath(lower, "oauth")` - Unlikely false positive

**Recommended:** Change `token` to `containsSegment`:
```zig
containsSegment(lower, "token") or    // was: containsPath
```

This prevents `tokenizer.ts`, `tokenize.go`, etc. from being classified as auth.

### 1.5 Add lockfile recognition

**Problem:** `pnpm-lock.yaml` is classified as `config` due to `.yaml` extension. Lock files are typically generated artifacts that accompany dependency changes. They inflate domain counts unnecessarily.

**Option A:** Treat lockfiles as a separate ignored domain (like `test`).
**Option B:** Classify lockfiles alongside their manifest (both as `config`).

Current behavior (Option B) is actually reasonable. No change needed.

### Summary of Quick Win Changes

| Fix | Lines Changed | Files Affected | Benchmark Cases Affected |
|-----|--------------|----------------|-------------------------|
| 1.1 `.json` → config | 1 line add | diff_parser.zig:95 | pass-007 (domain attribution) |
| 1.2 `session` → segment | 1 line edit | diff_parser.zig:59 | error-002 |
| 1.3 `schema` → segment | 1 line edit | diff_parser.zig:67 | error-001, error-002 |
| 1.4 `token` → segment | 1 line edit | diff_parser.zig:58 | (preventive) |

---

## Section 2: Tree-sitter Integration Design

### 2.1 Motivation

Path-based classification cannot determine:
- Whether `schemas.ts` is a Zod validation schema or a database schema
- Whether `session` in a filename refers to auth sessions or conference sessions
- What domain a generic `utils.go` or `helpers.py` belongs to
- Whether imports/exports indicate cross-domain coupling

Tree-sitter can parse diff `patch` content to extract semantic signals from the actual code changes.

### 2.2 Adding Tree-sitter as a Zig C Dependency

Tree-sitter provides a C API that Zig can consume directly via `@cImport`.

**build.zig changes:**
```zig
const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Tree-sitter core library
    const tree_sitter = b.dependency("tree-sitter", .{
        .target = target,
        .optimize = optimize,
    });

    // Language grammars
    const ts_typescript = b.dependency("tree-sitter-typescript", .{
        .target = target,
        .optimize = optimize,
    });
    const ts_python = b.dependency("tree-sitter-python", .{
        .target = target,
        .optimize = optimize,
    });
    const ts_go = b.dependency("tree-sitter-go", .{
        .target = target,
        .optimize = optimize,
    });

    const exe = b.addExecutable(.{
        .name = "gh-lint",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });

    exe.root_module.linkLibrary(tree_sitter.artifact("tree-sitter"));
    exe.root_module.linkLibrary(ts_typescript.artifact("tree-sitter-typescript"));
    exe.root_module.linkLibrary(ts_python.artifact("tree-sitter-python"));
    exe.root_module.linkLibrary(ts_go.artifact("tree-sitter-go"));

    b.installArtifact(exe);
}
```

**build.zig.zon** would need entries for each dependency pointing to their GitHub repos or a package index.

**Alternative approach:** Vendor the tree-sitter C source and grammar `.c` files directly under `deps/`, adding them as C source files in build.zig. This avoids Zig package registry dependencies and is more portable.

### 2.3 Grammars to Include

**Minimum viable set:**
1. **TypeScript/TSX** - Covers React ecosystem (supabase, next.js, shadcn-ui)
2. **Python** - Covers FastAPI ecosystem
3. **Go** - Covers many backend projects

**Phase 2:**
4. JavaScript/JSX
5. Rust
6. Zig (self-hosting)
7. SQL

### 2.4 Semantic Signals to Extract from Patch Content

The GitHub API's `patch` field (not currently fetched -- see Section 2.5) contains unified diff hunks. Tree-sitter can parse the added/removed lines to extract:

#### Signal 1: Import Analysis
```typescript
// If patch adds: import { z } from "zod"
// → schemas.ts is a validation file, NOT database
// If patch adds: import { prisma } from "@prisma/client"
// → schema.ts IS a database file
```

#### Signal 2: Function/Class Name Context
```typescript
// If patch modifies: export function SessionRecording() { ... }
// → "session" is a UI component name, NOT auth
// If patch modifies: async function createSession(userId: string) { ... }
// → "session" IS auth-related (takes userId)
```

#### Signal 3: Decorator/Attribute Analysis
```python
# If patch adds: @app.route("/api/users")
# → This is an API handler
# If patch adds: @pytest.fixture
# → This is a test file
```

#### Signal 4: Export Type Analysis
```go
// If patch modifies: type UserRepository struct { db *sql.DB }
// → database domain (contains sql.DB reference)
```

### 2.5 Fetching Patch Content

**Current state:** `PrFile` in `types.zig` does NOT have a `patch` field. The GitHub API returns it by default.

**Required change in `types.zig`:**
```zig
pub const PrFile = struct {
    filename: []const u8,
    status: []const u8,
    additions: u32,
    deletions: u32,
    changes: u32,
    patch: ?[]const u8 = null,  // unified diff content
};
```

**Required change in `pr_api.zig`:** Add `patch` to the deep copy:
```zig
result[i] = PrFile{
    // ... existing fields ...
    .patch = if (f.patch) |p| try alloc.dupe(u8, p) else null,
};
```

### 2.6 Architecture: Semantic Analyzer

**New file: `src/util/semantic_analyzer.zig`**

```
Input:  PrFile (with patch content)
Output: SemanticHints { domain_boost: ?Domain, domain_suppress: ?Domain, confidence: f32 }
```

**Integration with path-based classification:**

```
classifyFile(file: PrFile) -> Domain:
    path_domain = classifyPath(file.filename)           // existing
    if file.patch is null:
        return path_domain
    semantic = analyzeSemantics(file.filename, file.patch)  // new
    return reconcile(path_domain, semantic)
```

**Reconciliation rules:**
1. If `semantic.confidence > 0.8` and `semantic.domain_suppress == path_domain`: override to `semantic.domain_boost`
2. If `semantic.confidence > 0.5` and `path_domain == .unknown`: use `semantic.domain_boost`
3. Otherwise: keep `path_domain`

This ensures tree-sitter only overrides path classification when it has strong evidence.

### 2.7 New File Structure

```
src/
  util/
    diff_parser.zig           # existing, path-based classification
    semantic_analyzer.zig     # NEW: tree-sitter based semantic analysis
    file_classifier.zig       # NEW: combines path + semantic classification
  rules/
    detect_unscoped_change.zig  # updated to use file_classifier
  github/
    types.zig                 # updated: add patch field
    pr_api.zig                # updated: copy patch field
deps/
  tree-sitter/               # vendored C source (if not using zon)
  tree-sitter-typescript/
  tree-sitter-python/
  tree-sitter-go/
```

---

## Section 3: New Benchmark Cases

### 3.1 Cases for Quick Win Validation

#### Case: `pass-008` - Zod schema file only (fix 1.3)
```json
{
  "id": "pass-008",
  "description": "Zod validation schema update - should NOT trigger database domain",
  "repo": "supabase/supabase",
  "pr_number": null,
  "expected": "pass",
  "rationale": "schemas.ts with Zod imports is a validation file. After fix 1.3, 'schemas' no longer matches 'schema' segment. Falls to ui via .ts in component path, or unknown.",
  "category": "single-domain",
  "domains_expected": ["ui"],
  "ecosystem": "typescript",
  "files": [
    {"filename": "packages/marketing/src/components/forms/schemas.ts", "additions": 10, "deletions": 5, "changes": 15}
  ]
}
```

#### Case: `warn-005` - package.json mixed with API (fix 1.1)
```json
{
  "id": "warn-005",
  "description": "package.json update bundled with API route change - should warn",
  "expected": "warning",
  "rationale": "package.json = config (after fix 1.1). route.ts = api. 2 domains = WARNING.",
  "domains_expected": ["config", "api"],
  "files": [
    {"filename": "package.json", "additions": 5, "deletions": 2, "changes": 7},
    {"filename": "src/api/routes/users.ts", "additions": 20, "deletions": 10, "changes": 30}
  ]
}
```

### 3.2 Cases for Tree-sitter Validation

#### Case: `pass-009` - Conference session page (tree-sitter needed)
```json
{
  "id": "pass-009",
  "description": "Event page with 'session' in name - NOT auth, tree-sitter confirms UI component",
  "expected": "pass",
  "rationale": "stripe-sessions-contest.tsx contains React component JSX, no auth imports. Tree-sitter suppresses auth classification.",
  "domains_expected": ["ui"],
  "ecosystem": "typescript",
  "files": [
    {"filename": "apps/www/events/stripe-sessions-contest.tsx", "additions": 40, "deletions": 0, "changes": 40, "patch": "@@ -0,0 +1,40 @@\n+import React from 'react'\n+export default function SessionsContest() {\n+  return <div>...</div>\n+}"}
  ]
}
```

#### Case: `pass-010` - Go service single domain (tree-sitter needed)
```json
{
  "id": "pass-010",
  "description": "Go handler files - currently unknown, tree-sitter identifies as API",
  "expected": "pass",
  "rationale": "handler.go contains http.HandlerFunc, net/http imports. Tree-sitter classifies as api domain.",
  "domains_expected": ["api"],
  "ecosystem": "go",
  "files": [
    {"filename": "internal/server/handler.go", "additions": 15, "deletions": 5, "changes": 20, "patch": "@@ -10,5 +10,15 @@\n import \"net/http\"\n+func (s *Server) HandleUsers(w http.ResponseWriter, r *http.Request) {"}
  ]
}
```

#### Case: `warn-006` - Python mixed domains (tree-sitter needed)
```json
{
  "id": "warn-006",
  "description": "Python service mixing SQLAlchemy models with FastAPI routes",
  "expected": "warning",
  "rationale": "models.py imports sqlalchemy = database. main.py imports fastapi = api. Tree-sitter identifies both domains.",
  "domains_expected": ["database", "api"],
  "ecosystem": "python",
  "files": [
    {"filename": "app/models.py", "additions": 20, "deletions": 0, "changes": 20, "patch": "+from sqlalchemy import Column, Integer, String\n+class User(Base):"},
    {"filename": "app/main.py", "additions": 15, "deletions": 0, "changes": 15, "patch": "+from fastapi import FastAPI\n+app = FastAPI()\n+@app.get('/users')"}
  ]
}
```

#### Case: `pass-011` - Rust single domain (tree-sitter needed)
```json
{
  "id": "pass-011",
  "description": "Rust CLI utility - currently unknown, tree-sitter identifies as config/cli",
  "expected": "pass",
  "rationale": "main.rs and lib.rs with clap imports = CLI/config domain. Single domain.",
  "domains_expected": ["config"],
  "ecosystem": "rust",
  "files": [
    {"filename": "src/main.rs", "additions": 10, "deletions": 5, "changes": 15, "patch": "+use clap::Parser;\n+#[derive(Parser)]\n+struct Args {"},
    {"filename": "src/lib.rs", "additions": 8, "deletions": 3, "changes": 11}
  ]
}
```

#### Case: `error-004` - Tokenizer false positive prevention (fix 1.4)
```json
{
  "id": "error-004",
  "description": "NLP tokenizer + DB migration + UI - tokenizer.ts should NOT be auth",
  "expected": "error",
  "rationale": "tokenizer.ts should be unknown (not auth). migration/ = database. components/ .tsx = ui. After fix 1.4, only 2 active domains (database + ui) = WARNING, not ERROR.",
  "domains_expected": ["database", "ui"],
  "files": [
    {"filename": "src/nlp/tokenizer.ts", "additions": 30, "deletions": 10, "changes": 40},
    {"filename": "src/db/migration/001_add_users.sql", "additions": 15, "deletions": 0, "changes": 15},
    {"filename": "src/components/SearchBar.tsx", "additions": 20, "deletions": 5, "changes": 25}
  ]
}
```

---

## Implementation Priority

1. **Quick wins (Section 1)** - 4 line changes in `diff_parser.zig`, immediate improvement
2. **Add `patch` field to types** - Required foundation for tree-sitter
3. **Tree-sitter integration** - Largest effort, provides the most coverage improvement
4. **New benchmark cases** - Add alongside each implementation phase
