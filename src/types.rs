/// This is what your `r2md` logic uses for final output
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub rel_path: String,
    pub content: String,
}
