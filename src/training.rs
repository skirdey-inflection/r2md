use crate::FileEntry;
use anyhow::Result;
use serde::Serialize;
use std::fs::File;
use std::io::BufWriter;
use tiktoken_rs::cl100k_base;

#[derive(Serialize)]
struct TrainingSample {
    prompt: String,
    completion: String,
    prompt_tokens: usize,
    completion_tokens: usize,
    tokenizer: String,
    tokenizer_rs_version: String,
}

/// Produce a naive "80% prompt, 20% completion" from each file, then write all to a JSON array.
pub fn produce_training_json(files: &[FileEntry], out_path: &str) -> Result<()> {
    // For example, let’s do GPT-4 style tokenizer
    let bpe = cl100k_base()?;
    // You might store actual version from environment or just a literal "0.4"
    let tokenizer_version = "0.4";

    let mut samples = Vec::new();

    for file in files {
        // Convert entire file to tokens
        let tokens = bpe.encode_ordinary(&file.content);
        let total = tokens.len();
        if total < 2 {
            // if file is trivially small, skip or store minimal
            continue;
        }

        // We'll do an 80/20 split
        let prompt_end = (total as f64 * 0.8).ceil() as usize;
        let prompt_ids = &tokens[..prompt_end];
        let completion_ids = &tokens[prompt_end..];

        let prompt_str = bpe.decode(prompt_ids.to_vec()).unwrap_or_default();
        let completion_str = bpe.decode(completion_ids.to_vec()).unwrap_or_default();

        let sample = TrainingSample {
            prompt: prompt_str,
            completion: completion_str,
            prompt_tokens: prompt_ids.len(),
            completion_tokens: completion_ids.len(),
            tokenizer: "cl100k_base".to_string(),
            tokenizer_rs_version: tokenizer_version.to_string(),
        };
        samples.push(sample);
    }

    // Now write all samples to a JSON file
    let file = File::create(out_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &samples)?;

    Ok(())
}
