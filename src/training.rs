use crate::deps::sort_files_by_dependency;
use crate::types::FileEntry;
use anyhow::Result;
use serde::Serialize;
use std::fs::File;
use std::io::BufWriter;
use tokenizers::Tokenizer;

#[derive(Serialize)]
struct TrainingSample {
    prompt: String,
    completion: String,
    prompt_tokens: usize,
    completion_tokens: usize,
    tokenizer: String,
}

pub fn produce_training_json(files: &[FileEntry], out_path: &str, split_ratio: f64) -> Result<()> {
    // Validate split ratio
    if split_ratio <= 0.0 || split_ratio >= 1.0 {
        return Err(anyhow::anyhow!("Split ratio must be between 0 and 1"));
    }

    // Sort files by dependency
    let sorted_files = sort_files_by_dependency(files)?;
    let bpe = cl100k_base()?;

    let mut samples = Vec::new();

    for file in &sorted_files {
        let encoding = bpe.encode(file.content.as_str(), true).unwrap();

        let tokens = encoding.get_ids();
        let total = tokens.len();
        if total < 2 {
            continue; // Skip files that are too small
        }
        let prompt_end = (total as f64 * split_ratio).ceil() as usize;
        let prompt_ids = &tokens[..prompt_end];
        let completion_ids = &tokens[prompt_end..];
        let prompt_str = bpe.decode(prompt_ids, true).unwrap_or_default();
        let completion_str = bpe.decode(completion_ids, true).unwrap_or_default();
        let sample = TrainingSample {
            prompt: prompt_str,
            completion: completion_str,
            prompt_tokens: prompt_ids.len(),
            completion_tokens: completion_ids.len(),
            tokenizer: "deepseek-ai/DeepSeek-R1-Distill-Llama-70B".to_string(),
        };
        samples.push(sample);
    }

    let file = File::create(out_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &samples)?;

    Ok(())
}

fn cl100k_base() -> anyhow::Result<Tokenizer> {
    let tokenizer = Tokenizer::from_pretrained("deepseek-ai/DeepSeek-R1-Distill-Llama-70B", None)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(tokenizer)
}
