//! Scope classification logic for PR change analysis.
//!
//! Determines whether a PR's changes are well-scoped (single logical unit)
//! or spread across disconnected domains.

use creusot_std::macros::ensures;

use crate::verdict::Severity;

/// Classify the scope of a PR based on the number of connected components
/// among its changed code files.
#[ensures(code_files_count <= 1usize ==> result == Severity::Pass)]
#[ensures(code_files_count > 1usize && components <= 1usize ==> result == Severity::Pass)]
#[ensures(code_files_count > 1usize && components == 2usize ==> result == Severity::Warning)]
#[ensures(code_files_count > 1usize && components >= 3usize ==> result == Severity::Error)]
pub fn classify_scope(code_files_count: usize, components: usize) -> Severity {
    if code_files_count <= 1 {
        return Severity::Pass;
    }
    match components {
        0 | 1 => Severity::Pass,
        2 => Severity::Warning,
        _ => Severity::Error, // 3+
    }
}

/// Known non-code file extensions that should be excluded from scope analysis.
pub const NON_CODE_EXTENSIONS: &[&str] = &[
    ".md", ".rst", ".txt", ".json", ".yaml", ".yml", ".toml", ".lock", ".env", ".cfg", ".ini",
    ".css", ".scss", ".svg", ".png", ".jpg", ".gif", ".ico", ".woff", ".woff2",
];

/// Known non-code path prefixes that should be excluded from scope analysis.
pub const NON_CODE_PREFIXES: &[&str] = &[".github/", "docs/"];

/// Determine whether a file path refers to a non-code file.
pub fn is_non_code_file(filename: &str) -> bool {
    for prefix in NON_CODE_PREFIXES {
        if filename.starts_with(prefix) {
            return true;
        }
    }
    for ext in NON_CODE_EXTENSIONS {
        if filename.ends_with(ext) {
            return true;
        }
    }
    false
}

/// Resolve an import path against a set of changed file paths.
/// Returns the index of the matched file, if any.
pub fn resolve_import(import_path: &str, filenames: &[&str]) -> Option<usize> {
    let mut path = import_path;

    // Strip quotes (Go imports include them)
    if path.len() >= 2 && (path.starts_with('"') || path.starts_with('\'')) {
        path = &path[1..path.len() - 1];
    }

    // Strip relative prefixes
    if let Some(stripped) = path.strip_prefix("./") {
        path = stripped;
    } else if let Some(stripped) = path.strip_prefix("../") {
        path = stripped;
    } else if let Some(stripped) = path.strip_prefix("@/") {
        path = stripped;
    }

    // Convert Python dotted notation to path
    let converted: String;
    if path.contains('.') && !path.contains('/') {
        converted = path.replace('.', "/");
        path = &converted;
    } else {
        converted = String::new();
        let _ = &converted; // suppress unused warning
    }

    // Match against changed file names (suffix match)
    for (idx, fname) in filenames.iter().enumerate() {
        // Exact suffix match
        if fname.ends_with(path) {
            return Some(idx);
        }
        // Try with common extensions
        for ext in &[
            ".ts", ".tsx", ".js", ".jsx", ".py", ".go", "/index.ts", "/index.js",
        ] {
            let with_ext = format!("{path}{ext}");
            if fname.ends_with(&with_ext) {
                return Some(idx);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_or_one_file_is_pass() {
        assert_eq!(classify_scope(0, 0), Severity::Pass);
        assert_eq!(classify_scope(1, 1), Severity::Pass);
        assert_eq!(classify_scope(1, 5), Severity::Pass);
    }

    #[test]
    fn single_component_is_pass() {
        assert_eq!(classify_scope(5, 1), Severity::Pass);
    }

    #[test]
    fn two_components_is_warning() {
        assert_eq!(classify_scope(5, 2), Severity::Warning);
    }

    #[test]
    fn three_or_more_components_is_error() {
        assert_eq!(classify_scope(5, 3), Severity::Error);
        assert_eq!(classify_scope(10, 7), Severity::Error);
    }

    #[test]
    fn markdown_is_non_code() {
        assert!(is_non_code_file("README.md"));
        assert!(is_non_code_file("docs/guide.md"));
    }

    #[test]
    fn github_dir_is_non_code() {
        assert!(is_non_code_file(".github/workflows/ci.yml"));
    }

    #[test]
    fn source_files_are_code() {
        assert!(!is_non_code_file("src/main.rs"));
        assert!(!is_non_code_file("lib/utils.ts"));
        assert!(!is_non_code_file("app.py"));
    }

    #[test]
    fn resolve_relative_import() {
        let files = vec!["src/utils/helper.ts"];
        assert_eq!(resolve_import("./helper", &files), Some(0));
    }

    #[test]
    fn resolve_python_dotted() {
        let files = vec!["src/foo/bar.py"];
        assert_eq!(resolve_import("foo.bar", &files), Some(0));
    }

    #[test]
    fn resolve_go_quoted() {
        let files = vec!["internal/handler.go"];
        assert_eq!(resolve_import("\"internal/handler\"", &files), Some(0));
    }

    #[test]
    fn no_match_returns_none() {
        let files = vec!["src/main.rs"];
        assert_eq!(resolve_import("nonexistent", &files), None);
    }

    #[test]
    fn classify_scope_exhaustive_for_small_inputs() {
        for files in 0..=10 {
            for comps in 0..=10 {
                let result = classify_scope(files, comps);
                if files <= 1 {
                    assert_eq!(result, Severity::Pass, "files={files}, comps={comps}");
                } else {
                    match comps {
                        0 | 1 => assert_eq!(result, Severity::Pass, "files={files}, comps={comps}"),
                        2 => {
                            assert_eq!(result, Severity::Warning, "files={files}, comps={comps}")
                        }
                        _ => assert_eq!(result, Severity::Error, "files={files}, comps={comps}"),
                    }
                }
            }
        }
    }
}
