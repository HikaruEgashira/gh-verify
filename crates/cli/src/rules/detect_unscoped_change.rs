use anyhow::Result;
use gh_verify_core::scope::{classify_scope, is_non_code_file, resolve_import};
use gh_verify_core::union_find::{NodeKind, UnionFind};
use gh_verify_core::verdict::{RuleResult, Severity};

use crate::util::symbol_extractor;

use super::{Rule, RuleContext};

const RULE_ID: &str = "detect-unscoped-change";

pub struct DetectUnscopedChange;

impl Rule for DetectUnscopedChange {
    fn id(&self) -> &'static str {
        RULE_ID
    }

    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>> {
        let pr_files = match ctx {
            RuleContext::Pr { pr_files, .. } => pr_files,
            RuleContext::Release { .. } => return Ok(vec![pass_result()]),
        };

        // Filter to code files with patches
        let code_files: Vec<(u16, &crate::github::types::PrFile)> = pr_files
            .iter()
            .enumerate()
            .filter(|(_, f)| f.patch.is_some() && !is_non_code_file(&f.filename))
            .map(|(i, f)| (i as u16, f))
            .collect();

        // 0-1 code files: always scoped
        if code_files.len() <= 1 {
            return Ok(vec![pass_result()]);
        }

        // Extract symbols from each file
        let mut all_symbols = Vec::new();
        for (_, file) in &code_files {
            let symbols =
                symbol_extractor::extract_symbols(&file.filename, file.patch.as_deref().unwrap())?;
            all_symbols.push(symbols);
        }

        // Build call graph
        let mut graph = UnionFind::new();

        // Create file-level nodes
        let mut file_nodes = Vec::new();
        for (idx, (file_idx, file)) in code_files.iter().enumerate() {
            let node = graph.add_node(*file_idx, &file.filename, NodeKind::File);
            file_nodes.push(node);

            // Create definition nodes and merge with file node
            let symbols = &all_symbols[idx];
            for def_name in &symbols.definitions {
                let def_node = graph.add_node(*file_idx, def_name, NodeKind::Function);
                graph.merge(node, def_node);
            }
        }

        // Cross-file edges: call-to-definition matching
        for (idx_a, syms_a) in all_symbols.iter().enumerate() {
            for call_name in &syms_a.calls {
                for (idx_b, syms_b) in all_symbols.iter().enumerate() {
                    if idx_a == idx_b {
                        continue;
                    }
                    for def_name in &syms_b.definitions {
                        if call_name == def_name {
                            graph.merge(file_nodes[idx_a], file_nodes[idx_b]);
                        }
                    }
                }
            }
        }

        // Import edges: resolve import paths to changed files
        let filenames: Vec<&str> = code_files.iter().map(|(_, f)| f.filename.as_str()).collect();
        for (idx_a, syms_a) in all_symbols.iter().enumerate() {
            for import_path in &syms_a.imports {
                if let Some(idx_b) = resolve_import(import_path, &filenames) {
                    if idx_a != idx_b {
                        graph.merge(file_nodes[idx_a], file_nodes[idx_b]);
                    }
                }
            }
        }

        // Count connected components
        let components = graph.component_count();
        let severity = classify_scope(code_files.len(), components);

        if severity == Severity::Pass {
            return Ok(vec![pass_result()]);
        }

        // Build result with component details
        let comp_groups = graph.get_components();
        let mut affected = Vec::new();
        let mut detail = String::new();

        for (comp_idx, group) in comp_groups.iter().enumerate() {
            detail.push_str(&format!("  Component {}:", comp_idx + 1));
            for &file_idx in group {
                let filename = &pr_files[file_idx as usize].filename;
                detail.push_str(&format!(" {filename}"));
                affected.push(filename.clone());
            }
            detail.push('\n');
        }

        Ok(vec![RuleResult {
            rule_id: RULE_ID.to_string(),
            severity,
            message: format!("PR has {components} disconnected change clusters"),
            affected_files: affected,
            suggestion: Some(detail),
        }])
    }
}

fn pass_result() -> RuleResult {
    RuleResult::pass(RULE_ID, "PR is well-scoped")
}
