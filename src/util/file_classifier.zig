const diff_parser = @import("diff_parser.zig");
const semantic = @import("semantic_analyzer.zig");
const Domain = diff_parser.Domain;
const PrFile = @import("../github/types.zig").PrFile;

/// Determine file domain by combining path-based classification with semantic analysis.
pub fn classifyFile(file: PrFile) Domain {
    const path_domain = diff_parser.classifyPath(file.filename);

    const patch = file.patch orelse return path_domain;
    const hints = semantic.analyzeSemantics(file.filename, patch);

    // High confidence: override path-based classification with boosted domain
    if (hints.confidence > 0.8) {
        if (hints.domain_suppress) |suppress| {
            if (suppress == path_domain) {
                if (hints.domain_boost) |boost| return boost;
            }
        }
    }

    // When path-based result is unknown, accept boost at moderate confidence
    if (path_domain == .unknown and hints.confidence > 0.5) {
        if (hints.domain_boost) |boost| return boost;
    }

    return path_domain;
}
