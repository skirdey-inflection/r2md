use crate::types::CodeChunk;
use tree_sitter::{Language, Node, Parser};

// We'll need the actual extern "C" function from tree-sitter-python
#[link(name = "tree-sitter-python", kind = "static")]
extern "C" {
    fn tree_sitter_python() -> Language;
}

/// Parse a Python source file using Tree-sitter
/// Extract top-level function_definition and class_definition nodes
pub fn parse_python_tree(content: &str) -> Vec<CodeChunk> {
    let mut parser = Parser::new();

    // Call the function from tree-sitter-python
    let language = unsafe { tree_sitter_python() };
    parser
        .set_language(&language)
        .expect("Error loading Python grammar");

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => {
            // If parse fails, return entire content as fallback
            return vec![CodeChunk {
                text: content.to_string(),
                language: "python".to_string(),
            }];
        }
    };

    let root = tree.root_node();
    let mut results = Vec::new();

    // We'll do a shallow parse, just scanning direct children of the root.
    // If you want nested defs, do a recursive approach or a queue-based approach.
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        let kind = child.kind();
        if kind == "function_definition" || kind == "class_definition" {
            let snippet = extract_snippet(content, child);
            results.push(CodeChunk {
                text: snippet,
                language: "python".to_string(),
            });
        }
    }

    // If we found nothing, fallback to entire file
    if results.is_empty() {
        results.push(CodeChunk {
            text: content.to_string(),
            language: "python".to_string(),
        });
    }

    results
}

/// Helper to extract the substring from the entire source,
/// given a Tree-sitter Node
fn extract_snippet(source: &str, node: Node) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    source[start..end].to_string()
}
