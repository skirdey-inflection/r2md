use crate::types::CodeChunk;
use tree_sitter::{Language, Node, Parser};

#[link(name = "tree-sitter-typescript", kind = "static")]
extern "C" {
    fn tree_sitter_typescript() -> Language;
}

pub fn parse_typescript_tree(content: &str) -> Vec<CodeChunk> {
    let mut parser = Parser::new();

    let language = unsafe { tree_sitter_typescript() };
    parser
        .set_language(&language)
        .expect("Error loading TypeScript grammar");

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => {
            return vec![CodeChunk {
                text: content.to_string(),
                language: "typescript".to_string(),
            }];
        }
    };

    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut results = Vec::new();

    for child in root.children(&mut cursor) {
        let kind = child.kind();
        if matches!(kind, "function_declaration" | "class_declaration" | "interface_declaration") {
            let snippet = extract_snippet(content, child);
            results.push(CodeChunk {
                text: snippet,
                language: "typescript".to_string(),
            });
        }
    }

    if results.is_empty() {
        results.push(CodeChunk {
            text: content.to_string(),
            language: "typescript".to_string(),
        });
    }

    results
}

fn extract_snippet(source: &str, node: Node) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    source[start..end].to_string()
}
