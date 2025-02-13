mod python;
mod rustlang;
mod fallback;
mod javascript;
mod typescript;
mod java;
mod cpp;
// ... plus your existing rustlang, python, fallback, etc.

use crate::types::CodeChunk;

pub use javascript::parse_javascript_tree;
pub use typescript::parse_typescript_tree;
pub use java::parse_java_tree;
pub use cpp::parse_cpp_tree;
// pub use fallback::parse_fallback_line_based;
pub use python::parse_python_tree;
pub use fallback::parse_fallback_line_based;
pub use rustlang::parse_rust_tree;
// pub use rustlang::parse_rust_tree;

pub fn parse_file_to_chunks(content: &str, ext: &str) -> Vec<CodeChunk> {
    match ext {
        "py"  => parse_python_tree(content),
        "rs"  => parse_rust_tree(content),

        "js"  => parse_javascript_tree(content),
        "ts"  => parse_typescript_tree(content),
        "java" => parse_java_tree(content),
        // c++ can appear in multiple ext forms:
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "h" 
            => parse_cpp_tree(content),

        // everything else => fallback
        _ => parse_fallback_line_based(content, ext),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_dispatch() {
        let rust_code = "fn main() {}";
        let chunks = parse_file_to_chunks(rust_code, "rs");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].language, "rust");

        let python_code = "def foo(): pass";
        let py_chunks = parse_file_to_chunks(python_code, "py");
        assert!(!py_chunks.is_empty());
    }
}
