use crate::types::CodeChunk;
use tree_sitter::{Language, Node, Parser};

#[link(name = "tree-sitter-javascript", kind = "static")]
extern "C" {
    fn tree_sitter_javascript() -> Language;
}

pub fn parse_javascript_tree(content: &str) -> Vec<CodeChunk> {
    let mut parser = Parser::new();

    let language = unsafe { tree_sitter_javascript() };
    parser
        .set_language(&language)
        .expect("Error loading JavaScript grammar");

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => {
            return vec![CodeChunk {
                text: content.to_string(),
                language: "javascript".to_string(),
            }];
        }
    };

    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut results = Vec::new();

    // Naive top-level function/class detection
    for child in root.children(&mut cursor) {
        let kind = child.kind();
        if kind == "function_declaration" || kind == "class_declaration" {
            let snippet = extract_snippet(content, child);
            results.push(CodeChunk {
                text: snippet,
                language: "javascript".to_string(),
            });
        }
    }

    if results.is_empty() {
        results.push(CodeChunk {
            text: content.to_string(),
            language: "javascript".to_string(),
        });
    }

    results
}

fn extract_snippet(source: &str, node: Node) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    source[start..end].to_string()
}
