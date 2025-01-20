pub mod fallback;
pub mod python;
pub mod rustlang;

use crate::types::CodeChunk;
use fallback::parse_fallback_line_based;
use python::parse_python_tree;
use rustlang::parse_rust_tree;

pub fn parse_file_to_chunks(content: &str, ext: &str) -> Vec<CodeChunk> {
    match ext {
        "py" => parse_python_tree(content),
        "rs" => parse_rust_tree(content),
        _ => parse_fallback_line_based(content, ext),
    }
}
