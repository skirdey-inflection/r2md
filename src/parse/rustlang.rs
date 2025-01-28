// src/parse/rustlang.rs
use crate::types::CodeChunk;
use tree_sitter::{Language, Node, Parser};

#[link(name = "tree-sitter-rust", kind = "static")]
extern "C" {
    fn tree_sitter_rust() -> Language;
}

pub fn parse_rust_tree(content: &str) -> Vec<CodeChunk> {
    let mut parser = Parser::new();

    // This is the fix: pass a reference
    let language = unsafe { tree_sitter_rust() };
    parser
        .set_language(&language)
        .expect("Error loading Rust grammar");

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => {
            // fallback
            return vec![CodeChunk {
                text: content.to_string(),
                language: "rust".to_string(),
            }];
        }
    };

    let root = tree.root_node();
    let mut results = Vec::new();

    // Shallow parse for top-level items
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        let kind = child.kind();
        if matches!(
            kind,
            "function_item" | "struct_item" | "enum_item" | "impl_item" | "trait_item"
        ) {
            let snippet = extract_snippet(content, child);
            results.push(CodeChunk {
                text: snippet,
                language: "rust".to_string(),
            });
        }
    }

    if results.is_empty() {
        results.push(CodeChunk {
            text: content.to_string(),
            language: "rust".to_string(),
        });
    }

    results
}

fn extract_snippet(source: &str, node: Node) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    source[start..end].to_string()
}
