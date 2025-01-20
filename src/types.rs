#[derive(Debug)]
pub struct CodeChunk {
    pub text: String,
    pub language: String,
}

/// This is what your `r2md` logic uses for final output
#[derive(Debug)]
pub struct FileEntry {
    pub rel_path: String,
    pub content: String,
}
