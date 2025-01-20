use crate::types::CodeChunk;
use tiktoken_rs::{cl100k_base, CoreBPE};

/// Example: token-based splitting of large code chunks
pub fn tokenize_and_split(chunks: Vec<CodeChunk>, max_context: usize) -> Vec<CodeChunk> {
    // e.g. we load a tokenizer
    let bpe = cl100k_base().expect("Could not load cl100k_base tokenizer");
    let mut results = Vec::new();

    for chunk in chunks {
        let token_ids = bpe.encode_ordinary(&chunk.text);
        if token_ids.len() <= max_context {
            results.push(chunk);
        } else {
            // naive approach: break into slices
            let mut idx = 0;
            while idx < token_ids.len() {
                let end = (idx + max_context).min(token_ids.len());
                let sub = &token_ids[idx..end];
                let sub_str = bpe.decode(sub.to_vec()).unwrap_or_default();
                results.push(CodeChunk {
                    text: sub_str,
                    language: chunk.language.clone(),
                });
                idx = end;
            }
        }
    }
    results
}
