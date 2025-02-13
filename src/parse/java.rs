use crate::types::CodeChunk;
use tree_sitter::{Language, Node, Parser};

#[link(name = "tree-sitter-java", kind = "static")]
extern "C" {
    fn tree_sitter_java() -> Language;
}

pub fn parse_java_tree(content: &str) -> Vec<CodeChunk> {
    let mut parser = Parser::new();

    let language = unsafe { tree_sitter_java() };
    parser
        .set_language(&language)
        .expect("Error loading Java grammar");

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => {
            return vec![CodeChunk {
                text: content.to_string(),
                language: "java".to_string(),
            }];
        }
    };

    let root = tree.root_node();
    let mut results = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        let kind = child.kind();
        if matches!(kind, "class_declaration" | "interface_declaration" | "enum_declaration") {
            let snippet = extract_snippet(content, child);
            results.push(CodeChunk {
                text: snippet,
                language: "java".to_string(),
            });
        }
    }

    if results.is_empty() {
        results.push(CodeChunk {
            text: content.to_string(),
            language: "java".to_string(),
        });
    }

    results
}

fn extract_snippet(source: &str, node: Node) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    source[start..end].to_string()
}
