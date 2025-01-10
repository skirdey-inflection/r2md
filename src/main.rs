use clap::{Arg, Command};
use ignore::{DirEntry, WalkBuilder};
use walkdir::WalkDir;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct FileEntry {
    rel_path: String,
    content: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments using clap
    let matches = Command::new("repo2markdown")
        .version("0.3.0")
        .author("Your Name <you@example.com>")
        .about("Converts a local GitHub repository into a single Markdown file.")
        .arg(
            Arg::new("path")
                .help("Path to the repository (default: current directory)")
                .required(false)
                .index(1)
                .default_value("."),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file name (default: r2md_output.md)")
                .required(false),
        )
        .get_matches();

    let repo_path_str = matches.get_one::<String>("path").unwrap();
    let repo_path = PathBuf::from(repo_path_str);

    let output_file_name = matches
        .get_one::<String>("output")
        .map(|s| s.as_str())
        .unwrap_or("r2md_output.md");

    if !repo_path.is_dir() {
        eprintln!("Error: provided path is not a directory or doesn't exist.");
        std::process::exit(1);
    }

    // Collect files (respecting .gitignore, skipping unwanted files)
    let files = collect_files(&repo_path)?;

    // Generate the Markdown (including folder structure and code blocks)
    let markdown = generate_markdown(&repo_path, &files);

    // Write the output to disk
    write_output_file(&markdown, output_file_name)?;

    println!("Markdown exported to {}", output_file_name);
    Ok(())
}

fn collect_files(repo_path: &Path) -> Result<Vec<FileEntry>, Box<dyn Error>> {
    let mut files = Vec::new();
    let walker = WalkBuilder::new(repo_path)
        .hidden(false) // We'll do dotfile filtering ourselves
        .follow_links(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .build();

    for result in walker {
        match result {
            Ok(entry) => {
                if should_include(&entry) {
                    let path = entry.path();
                    let rel_path = path
                        .strip_prefix(repo_path)?
                        .to_string_lossy()
                        .to_string();
                    let content = fs::read_to_string(path)
                        .unwrap_or_else(|_| "Unable to read file. Possibly binary.".to_string());
                    files.push(FileEntry { rel_path, content });
                }
            }
            Err(e) => eprintln!("Error while traversing: {}", e),
        }
    }
    Ok(files)
}

/// Decide if we should include this file:
///  1. Skip directories (only need files).
///  2. Skip hidden files/directories (leading dot).
///  3. Skip certain well-known directories (venv, node_modules, etc.).
///  4. Skip certain file extensions (json, yaml, lock, etc.).
fn should_include(entry: &DirEntry) -> bool {
    let path = entry.path();
    // Skip directories
    if entry.file_type().map_or(false, |ft| ft.is_dir()) {
        return false;
    }

    let file_name = match path.file_name().and_then(OsStr::to_str) {
        Some(name) => name,
        None => return false,
    };

    // Skip dotfiles
    if file_name.starts_with('.') {
        return false;
    }

    // Ignore certain parent directories
    if path.components().any(|comp| {
        matches!(
            comp.as_os_str().to_str(),
            Some("venv")
                | Some(".venv")
                | Some("node_modules")
                | Some("__pycache__")
                | Some("dist")
                | Some("build")
                | Some("target")
                | Some(".git")
                | Some(".svn")
                | Some(".hg")
                | Some(".idea")
                | Some(".vscode")
        )
    }) {
        return false;
    }

    // Ignore certain file extensions
    if let Some(ext) = path.extension().and_then(OsStr::to_str) {
        if matches!(ext.to_lowercase().as_str(), "json" | "yaml" | "yml" | "lock" | "log") {
            return false;
        }
    }

    true
}

/// Combines the directory tree (with the root folder name at top) and
/// file contents into a single Markdown string.
fn generate_markdown(repo_path: &Path, files: &[FileEntry]) -> String {
    let mut md_output = String::new();

    // Title
    md_output.push_str("# Repository Markdown Export\n\n");

    // Directory structure
    md_output.push_str("## Directory Structure\n\n");
    let dir_tree = generate_directory_tree(repo_path);
    md_output.push_str("```\n");
    md_output.push_str(&dir_tree);
    md_output.push_str("```\n\n");

    // Code
    md_output.push_str("## Code\n\n");
    for file in files {
        md_output.push_str(&format!("### `{}`\n\n", file.rel_path));
        md_output.push_str("```plaintext\n");
        md_output.push_str(&file.content);
        md_output.push_str("\n```\n\n");
    }

    md_output
}

fn generate_directory_tree(repo_path: &Path) -> String {
    // First, get an absolute path instead of "."
    let canonical = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());

    // Try to use the last path segment as the root name.
    let root_name = match canonical.file_name().and_then(|s| s.to_str()) {
        Some(fname) => fname.to_string(),
        // If we can't get a file name (e.g. root `/`), fall back to full path as a string.
        None => canonical.to_string_lossy().to_string(),
    };

    // Start the output with the root folder line
    // Then walk sub-directories from min_depth(1) so we don't double-print the root
    let mut output = format!("- {}/\n", root_name);

    for entry in WalkDir::new(&canonical).min_depth(1) {
        if let Ok(e) = entry {
            let depth = e.depth();
            let path = e.path();

            // If the path should be skipped in the tree (hidden or ignored), continue
            if skip_in_tree(path) {
                continue;
            }

            // Indentation based on depth
            let indent = "  ".repeat(depth);

            // Convert the path to a relative path by stripping off the canonical root
            let rel_path = path
                .strip_prefix(&canonical)
                .unwrap_or(path) // fallback
                .to_string_lossy();

            // Print directories with a trailing slash
            if e.file_type().is_dir() {
                output.push_str(&format!("{}- {}/\n", indent, rel_path));
            } else {
                output.push_str(&format!("{}- {}\n", indent, rel_path));
            }
        }
    }

    output
}


/// Decide if this path should be skipped in the directory tree.
/// If it is a hidden or ignored directory, or inside an ignored directory, we skip it.
fn skip_in_tree(path: &Path) -> bool {
    for comp in path.components() {
        if let Some(c) = comp.as_os_str().to_str() {
            // If hidden folder/file or one of the known "ignored" directories
            if c.starts_with('.') {
                return true;
            }
            if matches!(
                c,
                "venv"
                    | ".venv"
                    | "node_modules"
                    | "__pycache__"
                    | "dist"
                    | "build"
                    | "target"
                    | ".git"
                    | ".svn"
                    | ".hg"
                    | ".idea"
                    | ".vscode"
            ) {
                return true;
            }
        }
    }
    false
}

fn write_output_file(markdown: &str, output_file_name: &str) -> io::Result<()> {
    let file = File::create(output_file_name)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(markdown.as_bytes())?;
    writer.flush()?;
    Ok(())
}
