const diff_parser = @import("diff_parser.zig");
const semantic = @import("semantic_analyzer.zig");
const Domain = diff_parser.Domain;
const PrFile = @import("../github/types.zig").PrFile;

/// パスベース分類とセマンティック分析を統合してファイルのドメインを決定する。
pub fn classifyFile(file: PrFile) Domain {
    const path_domain = diff_parser.classifyPath(file.filename);

    const patch = file.patch orelse return path_domain;
    const hints = semantic.analyzeSemantics(file.filename, patch);

    // 高信頼度でパスベース分類を抑制する場合はブーストドメインを採用
    if (hints.confidence > 0.8) {
        if (hints.domain_suppress) |suppress| {
            if (suppress == path_domain) {
                if (hints.domain_boost) |boost| return boost;
            }
        }
    }

    // パスベースが unknown の場合、中程度の信頼度でもブーストを採用
    if (path_domain == .unknown and hints.confidence > 0.5) {
        if (hints.domain_boost) |boost| return boost;
    }

    return path_domain;
}
