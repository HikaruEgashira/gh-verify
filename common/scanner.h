#ifndef TREE_SITTER_TYPESCRIPT_SCANNER_H_
#define TREE_SITTER_TYPESCRIPT_SCANNER_H_

#include "tree_sitter/parser.h"
#include <stdbool.h>

static inline bool external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
    (void)payload;
    (void)lexer;
    (void)valid_symbols;
    return false;
}

#endif
