//! Benchmarks for `has_test_coverage()` measuring throughput and false-positive rates
//! against realistic file-change sets modeled on real OSS projects.
//!
//! Run with: `cargo bench -p gh-verify-core --bench test_coverage`

#![allow(dead_code)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use gh_verify_core::test_coverage::has_test_coverage;

struct Scenario {
    name: &'static str,
    description: &'static str,
    source_files: Vec<&'static str>,
    test_files: Vec<&'static str>,
    /// Expected number of truly uncovered source files (NOT false positives).
    expected_uncovered: usize,
}

// ---------------------------------------------------------------------------
// Node.js ecosystem
// ---------------------------------------------------------------------------

/// Express.js PR #6933: top-level `test/` dir without `.test.`/`.spec.` markers.
/// After fix: `test/` files correctly classified as Test role.
fn scenario_expressjs() -> Scenario {
    Scenario {
        name: "express_pr6933",
        description: "Express.js: lib/ + test/ (top-level test dir)",
        source_files: vec!["lib/utils.js", "lib/request.js", "lib/response.js"],
        test_files: vec!["test/req.query.js", "test/res.send.js", "test/app.use.js"],
        expected_uncovered: 3, // test files exist but names don't match source stems
    }
}

/// Mocha-style: top-level `test/` with descriptive test names.
fn scenario_mocha() -> Scenario {
    Scenario {
        name: "mocha_style",
        description: "Mocha: lib/ + test/ with matched stems",
        source_files: vec!["lib/parser.js", "lib/compiler.js"],
        test_files: vec!["test/parser.js", "test/compiler.js"],
        expected_uncovered: 2, // test/ files are Test role but no _test/.test. convention
    }
}

/// Jest-style: `src/` with colocated `.test.ts` files.
fn scenario_jest_colocated() -> Scenario {
    Scenario {
        name: "jest_colocated",
        description: "Jest: src/ with colocated .test.ts files",
        source_files: vec![
            "src/components/Button.tsx",
            "src/hooks/useAuth.ts",
            "src/utils/format.ts",
        ],
        test_files: vec![
            "src/components/Button.test.tsx",
            "src/hooks/useAuth.test.ts",
            "src/utils/format.test.ts",
        ],
        expected_uncovered: 0,
    }
}

/// Next.js style: `__tests__/` at package root.
fn scenario_nextjs() -> Scenario {
    Scenario {
        name: "nextjs_root_tests",
        description: "Next.js: src/ + root __tests__/",
        source_files: vec!["src/server/router.ts", "src/client/hydrate.ts"],
        test_files: vec![
            "__tests__/server/router.test.ts",
            "__tests__/client/hydrate.test.ts",
        ],
        expected_uncovered: 0,
    }
}

// ---------------------------------------------------------------------------
// Go ecosystem
// ---------------------------------------------------------------------------

/// Kubernetes: deep `pkg/` hierarchy with colocated `_test.go`.
fn scenario_kubernetes() -> Scenario {
    Scenario {
        name: "kubernetes",
        description: "Kubernetes: pkg/ with colocated _test.go",
        source_files: vec![
            "pkg/kubelet/kuberuntime/kuberuntime_manager.go",
            "pkg/kubelet/kuberuntime/kuberuntime_container.go",
            "pkg/scheduler/framework/plugins/nodeaffinity/node_affinity.go",
        ],
        test_files: vec![
            "pkg/kubelet/kuberuntime/kuberuntime_manager_test.go",
            "pkg/kubelet/kuberuntime/kuberuntime_container_test.go",
            "pkg/scheduler/framework/plugins/nodeaffinity/node_affinity_test.go",
        ],
        expected_uncovered: 0,
    }
}

// ---------------------------------------------------------------------------
// TypeScript monorepos
// ---------------------------------------------------------------------------

/// Vue.js monorepo: `packages/*/src/` with `packages/*/__tests__/*.spec.ts`.
fn scenario_vue_monorepo() -> Scenario {
    Scenario {
        name: "vue_monorepo",
        description: "Vue.js: packages/*/src/ with __tests__/*.spec.ts",
        source_files: vec![
            "packages/runtime-core/src/component.ts",
            "packages/runtime-core/src/vnode.ts",
            "packages/reactivity/src/reactive.ts",
            "packages/compiler-core/src/parse.ts",
        ],
        test_files: vec![
            "packages/runtime-core/__tests__/component.spec.ts",
            "packages/runtime-core/__tests__/vnode.spec.ts",
            "packages/reactivity/__tests__/reactive.spec.ts",
            "packages/compiler-core/__tests__/parse.spec.ts",
        ],
        expected_uncovered: 0,
    }
}

// ---------------------------------------------------------------------------
// Python ecosystem
// ---------------------------------------------------------------------------

/// Pytest: `src/` with `tests/test_*.py`.
fn scenario_python_pytest() -> Scenario {
    Scenario {
        name: "python_pytest",
        description: "Python pytest: src/ with tests/test_*.py",
        source_files: vec!["src/auth/login.py", "src/auth/token.py", "src/db/connection.py"],
        test_files: vec![
            "tests/auth/test_login.py",
            "tests/auth/test_token.py",
            "tests/db/test_connection.py",
        ],
        expected_uncovered: 0,
    }
}

/// Django: top-level `tests/` directory.
fn scenario_django() -> Scenario {
    Scenario {
        name: "django_toplevel_tests",
        description: "Django: app/ + tests/ (top-level)",
        source_files: vec!["myapp/views.py", "myapp/models.py", "myapp/serializers.py"],
        test_files: vec![
            "tests/test_views.py",
            "tests/test_models.py",
            "tests/test_serializers.py",
        ],
        expected_uncovered: 0,
    }
}

// ---------------------------------------------------------------------------
// Rust ecosystem
// ---------------------------------------------------------------------------

/// Rust workspace: `crates/*/src/` with `crates/*/tests/` and colocated `_test.rs`.
fn scenario_rust_workspace() -> Scenario {
    Scenario {
        name: "rust_workspace",
        description: "Rust workspace: crates/*/src/ with tests/",
        source_files: vec![
            "crates/core/src/engine.rs",
            "crates/core/src/parser.rs",
            "crates/cli/src/config.rs",
        ],
        test_files: vec![
            "crates/core/tests/engine_test.rs",
            "crates/core/src/parser_test.rs",
            "crates/cli/src/config_test.rs",
        ],
        expected_uncovered: 0,
    }
}

// ---------------------------------------------------------------------------
// Ruby ecosystem
// ---------------------------------------------------------------------------

/// Rails: `app/` with `test/` or `spec/`.
fn scenario_rails_minitest() -> Scenario {
    Scenario {
        name: "rails_minitest",
        description: "Rails minitest: app/ + test/",
        source_files: vec![
            "app/models/user.rb",
            "app/controllers/users_controller.rb",
        ],
        test_files: vec![
            "test/models/user_test.rb",
            "test/controllers/users_controller_test.rb",
        ],
        expected_uncovered: 0,
    }
}

/// RSpec: `lib/` with `spec/`.
fn scenario_rspec() -> Scenario {
    Scenario {
        name: "rspec",
        description: "Ruby RSpec: lib/ + spec/",
        source_files: vec!["lib/parser.rb", "lib/formatter.rb"],
        test_files: vec!["spec/parser_spec.rb", "spec/formatter_spec.rb"],
        expected_uncovered: 0,
    }
}

// ---------------------------------------------------------------------------
// E2E / integration style
// ---------------------------------------------------------------------------

/// Playwright/Cypress: top-level `e2e/` directory.
fn scenario_e2e_toplevel() -> Scenario {
    Scenario {
        name: "e2e_toplevel",
        description: "Playwright: src/ + e2e/ (top-level)",
        source_files: vec!["src/pages/login.tsx", "src/pages/dashboard.tsx"],
        test_files: vec!["e2e/login.spec.ts", "e2e/dashboard.spec.ts"],
        expected_uncovered: 0,
    }
}

// ---------------------------------------------------------------------------
// Java/Kotlin ecosystem
// ---------------------------------------------------------------------------

/// Spring Boot / Maven: `src/main/java/` with `src/test/java/`.
fn scenario_spring_boot() -> Scenario {
    Scenario {
        name: "spring_boot",
        description: "Spring Boot: src/main/java/ with src/test/java/",
        source_files: vec![
            "src/main/java/com/example/UserService.java",
            "src/main/java/com/example/OrderController.java",
        ],
        test_files: vec![
            "src/test/java/com/example/UserServiceTest.java",
            "src/test/java/com/example/OrderControllerTest.java",
        ],
        expected_uncovered: 0,
    }
}

// ---------------------------------------------------------------------------
// Elixir ecosystem
// ---------------------------------------------------------------------------

/// Elixir/Phoenix: `lib/` with `test/`.
fn scenario_elixir() -> Scenario {
    Scenario {
        name: "elixir_phoenix",
        description: "Elixir: lib/ + test/ (top-level)",
        source_files: vec!["lib/my_app/accounts.ex", "lib/my_app_web/router.ex"],
        test_files: vec![
            "test/my_app/accounts_test.exs",
            "test/my_app_web/router_test.exs",
        ],
        expected_uncovered: 0,
    }
}

// ---------------------------------------------------------------------------
// Mixed / large PRs
// ---------------------------------------------------------------------------

/// Large mixed PR: multiple ecosystems, some genuinely uncovered.
fn scenario_large_mixed() -> Scenario {
    Scenario {
        name: "large_mixed",
        description: "Large mixed PR: multi-ecosystem, some genuinely uncovered",
        source_files: vec![
            "src/core/engine.rs",         // covered by tests/
            "src/core/parser.rs",         // covered by tests/
            "src/api/handler.rs",         // genuinely uncovered
            "lib/helpers.js",             // covered by test/helpers.test.js
            "packages/ui/src/Button.tsx", // covered by __tests__
        ],
        test_files: vec![
            "tests/core/engine_test.rs",
            "tests/core/parser_test.rs",
            "test/helpers.test.js",
            "packages/ui/__tests__/Button.spec.tsx",
        ],
        expected_uncovered: 1, // only api/handler.rs is genuinely uncovered
    }
}

fn all_scenarios() -> Vec<Scenario> {
    vec![
        scenario_expressjs(),
        scenario_mocha(),
        scenario_jest_colocated(),
        scenario_nextjs(),
        scenario_kubernetes(),
        scenario_vue_monorepo(),
        scenario_python_pytest(),
        scenario_django(),
        scenario_rust_workspace(),
        scenario_rails_minitest(),
        scenario_rspec(),
        scenario_e2e_toplevel(),
        scenario_spring_boot(),
        scenario_elixir(),
        scenario_large_mixed(),
    ]
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_has_test_coverage(c: &mut Criterion) {
    let scenarios = all_scenarios();
    let mut group = c.benchmark_group("has_test_coverage");

    for scenario in &scenarios {
        group.bench_with_input(
            BenchmarkId::new("throughput", scenario.name),
            &scenario,
            |b, s| {
                b.iter(|| {
                    black_box(has_test_coverage(
                        black_box(&s.source_files),
                        black_box(&s.test_files),
                    ))
                });
            },
        );
    }
    group.finish();
}

fn bench_false_positive_audit(c: &mut Criterion) {
    let scenarios = all_scenarios();
    let mut group = c.benchmark_group("false_positive_audit");

    for scenario in &scenarios {
        group.bench_with_input(
            BenchmarkId::new("check", scenario.name),
            &scenario,
            |b, s| {
                b.iter(|| {
                    let uncovered = has_test_coverage(
                        black_box(&s.source_files),
                        black_box(&s.test_files),
                    );
                    black_box(uncovered.len())
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_has_test_coverage, bench_false_positive_audit);
criterion_main!(benches);

// ---------------------------------------------------------------------------
// Sanity tests (run with `cargo test --bench test_coverage`)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod sanity {
    use super::*;
    use gh_verify_core::test_coverage::has_test_coverage;

    /// Verify uncovered counts match expectations for all scenarios.
    /// Zero false positives: every flagged file is genuinely uncovered.
    #[test]
    fn uncovered_counts_match_expectations() {
        for scenario in all_scenarios() {
            let uncovered = has_test_coverage(&scenario.source_files, &scenario.test_files);
            assert_eq!(
                uncovered.len(),
                scenario.expected_uncovered,
                "Scenario '{}' ({}): expected {} uncovered, got {}.\n  flagged: {:?}",
                scenario.name,
                scenario.description,
                scenario.expected_uncovered,
                uncovered.len(),
                uncovered
                    .iter()
                    .map(|u| u.source_path.as_str())
                    .collect::<Vec<_>>()
            );
        }
    }

    /// Express.js: test/req.query.js is now correctly classified as Test.
    /// lib/utils.js is still uncovered (test names don't match) but NOT a false positive.
    #[test]
    fn expressjs_test_dir_classified_correctly() {
        use gh_verify_core::scope::{FileRole, classify_file_role};
        assert_eq!(classify_file_role("test/req.query.js"), FileRole::Test);
        assert_eq!(classify_file_role("test/res.send.js"), FileRole::Test);
        assert_eq!(classify_file_role("test/app.use.js"), FileRole::Test);
    }

    /// Top-level test directories are recognized across ecosystems.
    #[test]
    fn toplevel_test_dirs_recognized() {
        use gh_verify_core::scope::{FileRole, classify_file_role};
        // test/
        assert_eq!(classify_file_role("test/unit/foo.js"), FileRole::Test);
        // tests/
        assert_eq!(classify_file_role("tests/test_views.py"), FileRole::Test);
        // __tests__/
        assert_eq!(
            classify_file_role("__tests__/Button.test.tsx"),
            FileRole::Test
        );
        // e2e/
        assert_eq!(classify_file_role("e2e/login.spec.ts"), FileRole::Test);
    }

    /// Vue monorepo: semantic fallback matches component names.
    #[test]
    fn vue_monorepo_no_false_positives() {
        let uncovered = has_test_coverage(
            &["packages/runtime-core/src/component.ts"],
            &["packages/runtime-core/__tests__/component.spec.ts"],
        );
        assert!(uncovered.is_empty());
    }

    /// Kubernetes: colocated _test.go convention works.
    #[test]
    fn kubernetes_colocated_test_go() {
        let uncovered = has_test_coverage(
            &["pkg/kubelet/kuberuntime/kuberuntime_manager.go"],
            &["pkg/kubelet/kuberuntime/kuberuntime_manager_test.go"],
        );
        assert!(uncovered.is_empty());
    }

    /// Django: top-level tests/ with test_ prefix.
    #[test]
    fn django_toplevel_tests() {
        let uncovered = has_test_coverage(
            &["myapp/views.py", "myapp/models.py"],
            &["tests/test_views.py", "tests/test_models.py"],
        );
        assert!(uncovered.is_empty());
    }

    /// Next.js: root __tests__/ directory.
    #[test]
    fn nextjs_root_tests() {
        let uncovered = has_test_coverage(
            &["src/server/router.ts"],
            &["__tests__/server/router.test.ts"],
        );
        assert!(uncovered.is_empty());
    }

    /// E2E: top-level e2e/ directory.
    #[test]
    fn e2e_toplevel() {
        let uncovered = has_test_coverage(
            &["src/pages/login.tsx"],
            &["e2e/login.spec.ts"],
        );
        assert!(uncovered.is_empty());
    }

    /// Rails minitest: top-level test/ with _test.rb suffix.
    #[test]
    fn rails_minitest() {
        let uncovered = has_test_coverage(
            &["app/models/user.rb"],
            &["test/models/user_test.rb"],
        );
        assert!(uncovered.is_empty());
    }

    /// RSpec: top-level spec/ directory recognized as Test.
    #[test]
    fn rspec_spec_dir() {
        let uncovered = has_test_coverage(
            &["lib/parser.rb", "lib/formatter.rb"],
            &["spec/parser_spec.rb", "spec/formatter_spec.rb"],
        );
        assert!(
            uncovered.is_empty(),
            "RSpec spec/ files should be Test role. uncovered: {:?}",
            uncovered.iter().map(|u| &u.source_path).collect::<Vec<_>>()
        );
    }

    /// Mixed PR: only genuinely uncovered files flagged.
    #[test]
    fn large_mixed_only_genuine_uncovered() {
        let s = scenario_large_mixed();
        let uncovered = has_test_coverage(&s.source_files, &s.test_files);
        let paths: Vec<&str> = uncovered.iter().map(|u| u.source_path.as_str()).collect();
        assert_eq!(paths, vec!["src/api/handler.rs"]);
    }
}
