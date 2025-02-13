use crate::types::CodeChunk;
use tree_sitter::{Language, Node, Parser};

#[link(name = "tree-sitter-cpp", kind = "static")]
extern "C" {
    fn tree_sitter_cpp() -> Language;
}

pub fn parse_cpp_tree(content: &str) -> Vec<CodeChunk> {
    let mut parser = Parser::new();

    let language = unsafe { tree_sitter_cpp() };
    parser
        .set_language(&language)
        .expect("Error loading C++ grammar");

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => {
            return vec![CodeChunk {
                text: content.to_string(),
                language: "cpp".to_string(),
            }];
        }
    };

    let root = tree.root_node();
    let mut results = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        let kind = child.kind();
        if matches!(
            kind,
            "function_definition"
                | "class_specifier"
                | "struct_specifier"
                | "namespace_definition"
        ) {
            let snippet = extract_snippet(content, child);
            results.push(CodeChunk {
                text: snippet,
                language: "cpp".to_string(),
            });
        }
    }

    if results.is_empty() {
        results.push(CodeChunk {
            text: content.to_string(),
            language: "cpp".to_string(),
        });
    }

    results
}

fn extract_snippet(source: &str, node: Node) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    source[start..end].to_string()
}

#[test]
fn test_cpp_parsing() {
    let code = r#"
    namespace MyApp {
        class MyClass {};
    }
    
    void foo() {} // Top-level function
    "#;
    
    let chunks = parse_cpp_tree(code);
    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].text.contains("namespace MyApp"));
    assert!(chunks[1].text.contains("void foo()"));
}