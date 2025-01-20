use crate::types::CodeChunk;

/// Extremely naive fallback: we do line-based splitting if we see "function", "class", or "def"
/// This is just an example for languages other than Python
pub fn parse_fallback_line_based(content: &str, lang: &str) -> Vec<CodeChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let mut results = Vec::new();
    let mut current_acc = String::new();

    for line in lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with("function")
            || trimmed.starts_with("class")
            || trimmed.starts_with("def ")
        {
            if !current_acc.is_empty() {
                results.push(CodeChunk {
                    text: current_acc.clone(),
                    language: lang.to_string(),
                });
                current_acc.clear();
            }
        }
        current_acc.push_str(line);
        current_acc.push('\n');
    }

    // final chunk
    if !current_acc.is_empty() {
        results.push(CodeChunk {
            text: current_acc.clone(),
            language: lang.to_string(),
        });
    }

    results
}
