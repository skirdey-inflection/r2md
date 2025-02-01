use atty; // for checking if stdout is a TTY
use clap::{Arg, ArgAction, Command};
use ignore::WalkBuilder;
use printpdf::{BuiltinFont, Mm, PdfDocument};
use serde::Deserialize;
use serde_yaml;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod training; // at the top
use crate::training::produce_training_json;

mod parse;
mod types;

use types::FileEntry;

/// Keep the original ~20 recognized language extensions (focusing on text-based code)
static RECOGNIZED_EXTENSIONS: &[&str] = &[
    // Rust
    "rs", // Python
    "py", // JavaScript
    "js", // TypeScript
    "ts", // C
    "c", "h", // C++
    "cpp", "hpp", "cc", "cxx", "hh",    // Java
    "java",  // C#
    "cs",    // Go
    "go",    // Ruby
    "rb",    // PHP
    "php",   // Swift
    "swift", // Kotlin
    "kt", "kts", // Objective-C
    "m",   // Objective-C++
    "mm",  // Shell scripts
    "sh",  // Batch
    "bat", // F#
    "fs",  // Visual Basic
    "vb",  // Scala
    "scala",
];

/// Built-in known "binary" file extensions we skip entirely
static BINARY_FILE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "exe", "dll", "so", "dylib", "pdf", "mp4", "mov", "zip", "tar",
    "gz", "bz2", "7z", "class", "jar", "psd", "obj", "lib", "a", "iso", "ico", "ttf", "woff",
    "woff2", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "apk", "msi", "o", "out", "bin", "map",
    "lock", "pkl", "npy", "rdata",
];

/// Known dependency or hidden folders to skip entirely
static SKIP_FOLDERS: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    ".idea",
    ".vscode",
    "node_modules",
    "target",
    ".fingerprint",
    "build",
    "dist",
    "venv",
    ".venv",
    "__pycache__",
    "bin",
    "obj",
    "out",
    "vendor",
];

/// Default maximum file size (5MB) for skipping large files
const DEFAULT_MAX_FILE_SIZE: u64 = 5 * 1024 * 1024;

// Helper: determine a language identifier from the file’s extension.
fn language_from_path(path: &Path) -> &str {
    match path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "java" => "java",
        "c" => "c",
        "cpp" => "cpp",
        other => {
            // You can add additional mappings here
            if other.is_empty() {
                "plaintext"
            } else {
                "unknwon"
            }
        }
    }
}

/// Config for optional YAML (`r2md.yml` / `r2md.yaml`)
#[derive(Debug, Deserialize)]
struct R2mdConfig {
    /// Additional ignore patterns (substring matches).
    #[serde(default)]
    ignore_patterns: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // (The unchanged CLI/argument parsing and config loading code remains here.)
    let matches = Command::new("r2md")
        .version("0.6.0")
        .author("Example <you@example.com>")
        .about("r2md: merges code from multiple directories, streams or writes Markdown, and can optionally produce PDF.")
        .arg(
            Arg::new("paths")
                .help("One or more directories to process")
                .num_args(0..)
                .default_value(".")
        )
        .arg(
            Arg::new("exclude")
                .short('x')
                .long("exclude")
                .help("Exclude the given folder (and subfolders) from processing")
                .action(ArgAction::Append)
                .required(false)
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output Markdown file name (default: r2md_output.md if not streaming)")
                .required(false),
        )
        .arg(
            Arg::new("pdf")
                .short('p')
                .long("pdf")
                .help("Produce a PDF file as well (default r2md_output.pdf)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("debug")
                .long("debug")
                .help("Enable debug output")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("train-json")
                .long("train-json")
                .value_name("FILE")
                .help("Write JSON training data to FILE (prompt+completion pairs)")
                .required(false),
        )
        .get_matches();

    // (Directory, excludes, streaming and config code unchanged)
    let directories: Vec<PathBuf> = matches
        .get_many::<String>("paths")
        .unwrap_or_default()
        .map(PathBuf::from)
        .collect();
    let excludes: Vec<PathBuf> = matches
        .get_many::<String>("exclude")
        .unwrap_or_default()
        .map(PathBuf::from)
        .collect();
    let stdout_is_tty = atty::is(atty::Stream::Stdout);
    let streaming = !stdout_is_tty;
    let output_md_file = matches
        .get_one::<String>("output")
        .map(|s| s.as_str())
        .unwrap_or("r2md_output.md");
    let produce_pdf = matches.get_flag("pdf");

    let config = load_config_file()?;
    let mut user_ignores = vec![];
    if let Some(ref c) = config {
        user_ignores.extend(c.ignore_patterns.clone());
    }
    let debug_mode = matches.get_flag("debug");

    let mut all_files = Vec::new();
    for dir in &directories {
        let collected = collect_files(dir, &user_ignores, &excludes, debug_mode)?;
        all_files.extend(collected);
    }

    if streaming {
        stream_markdown(&all_files)?;
        return Ok(());
    }

    // Build the Markdown output with proper code fences.
    let mut md_output = String::new();
    for dir in &directories {
        md_output.push_str("# Repository Markdown Export\n\n");
        md_output.push_str("## Directory Structure\n\n");
        md_output.push_str("```\n");
        md_output.push_str(&generate_directory_tree(dir)?);
        md_output.push_str("```\n\n");
    }
    md_output.push_str("## Code\n\n");
    for file in &all_files {
        let path = Path::new(&file.rel_path);
        let lang = language_from_path(path);
        let heading = format!("### `{}`\n\n", file.rel_path);
        md_output.push_str(&heading);
        md_output.push_str(&format!("```{}\n", lang));
        md_output.push_str(&file.content);
        md_output.push_str("\n```\n\n");
    }

    {
        let mut f = BufWriter::new(File::create(output_md_file)?);
        f.write_all(md_output.as_bytes())?;
        f.flush()?;
    }
    println!("Markdown exported to {}", output_md_file);

    if produce_pdf {
        let pdf_name = if output_md_file == "r2md_output.md" {
            "r2md_output.pdf".to_string()
        } else {
            output_md_file.replace(".md", ".pdf")
        };
        write_pdf_file(&all_files, &directories, &pdf_name)?;
        println!("PDF exported to {}", pdf_name);
    }

    if let Some(json_path) = matches.get_one::<String>("train-json") {
        produce_training_json(&all_files, json_path)?;
    }

    Ok(())
}

fn stream_markdown(files: &[FileEntry]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    writeln!(handle, "# r2md Streaming Output\n")?;
    for file in files {
        let path = Path::new(&file.rel_path);
        let lang = language_from_path(path);
        writeln!(handle, "### `{}`\n", file.rel_path)?;
        writeln!(handle, "```{}", lang)?;
        writeln!(handle, "{}", file.content)?;
        writeln!(handle, "```")?;
        writeln!(handle)?;
    }
    handle.flush()
}

fn write_pdf_file(
    files: &[FileEntry],
    directories: &[PathBuf],
    output_file_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use printpdf::{BuiltinFont, Color, Mm, PdfDocument, Rgb};
    use syntect::easy::HighlightLines;
    use syntect::highlighting::ThemeSet;
    use syntect::parsing::SyntaxSet;

    // Create a new PDF document.
    let (doc, page1, layer1) = PdfDocument::new("r2md PDF", Mm(297.0), Mm(210.0), "Layer 1");
    let font = doc.add_builtin_font(BuiltinFont::Courier)?;
    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    let mut current_y = 210.0_f32;

    // Prepare syntect’s syntax and theme sets.
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    // Choose a theme – here we use "InspiredGitHub".
    let theme = &ts.themes["InspiredGitHub"];

    // Print directory headers.
    for d in directories {
        if current_y < 20.0 {
            let (p, l) = doc.add_page(Mm(297.0), Mm(210.0), "Layer next");
            current_layer = doc.get_page(p).get_layer(l);
            current_y = 210.0;
        }
        let text = format!("Directory: {}\n", d.display());
        current_layer.use_text(text, 12.0, Mm(10.0), Mm(current_y), &font);
        current_y -= 10.0;
    }

    // For each file...
    for file in files {
        if current_y < 20.0 {
            let (p, l) = doc.add_page(Mm(297.0), Mm(210.0), "Layer next");
            current_layer = doc.get_page(p).get_layer(l);
            current_y = 210.0;
        }
        let heading = format!("File: {}\n", file.rel_path);
        current_layer.use_text(heading, 10.0, Mm(10.0), Mm(current_y), &font);
        current_y -= 6.0;

        // Determine syntax for highlighting.
        let path = std::path::Path::new(&file.rel_path);
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let syntax = ss
            .find_syntax_by_extension(ext)
            .unwrap_or_else(|| ss.find_syntax_plain_text());
        let mut highlighter = HighlightLines::new(syntax, theme);

        // Print file content line by line with token-level highlighting.
        for line in file.content.lines() {
            if current_y < 10.0 {
                let (p, l) = doc.add_page(Mm(297.0), Mm(210.0), "Layer next");
                current_layer = doc.get_page(p).get_layer(l);
                current_y = 210.0;
            }
            let regions = highlighter
                .highlight_line(line, &ss)
                .map_err(|e| format!("Highlighting error: {}", e))?;
            let mut x = Mm(10.0);
            // For each highlighted region, set the fill color and draw the text.
            for (style, text) in regions {
                let r = style.foreground.r as f32 / 255.0;
                let g = style.foreground.g as f32 / 255.0;
                let b = style.foreground.b as f32 / 255.0;
                current_layer.set_fill_color(Color::Rgb(Rgb::new(r, g, b, None)));
                current_layer.use_text(text, 8.0, x, Mm(current_y), &font);
                // Estimate width per token (using Courier: ~4.0 mm per character).
                let token_width = 1.7_f32 * (text.len() as f32);
                x += Mm(token_width);
            }
            current_y -= 4.0;
        }
        current_y -= 4.0; // extra gap between files
    }

    // Save the PDF document.
    doc.save(&mut std::io::BufWriter::new(std::fs::File::create(
        output_file_name,
    )?))?;
    Ok(())
}

/// Attempt to load config from r2md.yml or r2md.yaml, returning None if not found.
fn load_config_file() -> Result<Option<R2mdConfig>, Box<dyn Error>> {
    for candidate in &["r2md.yml", "r2md.yaml"] {
        if Path::new(candidate).exists() {
            let text = fs::read_to_string(candidate)?;
            let config: R2mdConfig = serde_yaml::from_str(&text)?;
            eprintln!("Loaded config from {}", candidate);
            return Ok(Some(config));
        }
    }
    Ok(None)
}

/// Return a simple directory tree (skip hidden/dep folders)
fn generate_directory_tree(dir: &Path) -> Result<String, Box<dyn Error>> {
    let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let root_name = canonical
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(".");

    let mut output = format!("- {}/\n", root_name);
    for entry in WalkDir::new(&canonical).min_depth(1) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let depth = entry.depth();
        let path = entry.path();

        // Skip if any component of the path starts with a dot
        if path.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .map_or(false, |s| s.starts_with('.'))
        }) {
            continue;
        }

        // Skip hidden or dependency folders
        if should_skip_folder(path) {
            continue;
        }
        // For files, skip if recognized as "skip" for us
        if !path.is_dir() && should_skip_file(path, &[], false) {
            continue;
        }

        let rel = path.strip_prefix(&canonical).unwrap_or(path);
        let indent = "  ".repeat(depth);
        if entry.file_type().is_dir() {
            output.push_str(&format!("{}- {}/\n", indent, rel.display()));
        } else {
            output.push_str(&format!("{}- {}\n", indent, rel.display()));
        }
    }

    Ok(output)
}

/// Determine if folder should be skipped (hidden or in SKIP_FOLDERS)
fn should_skip_folder(path: &Path) -> bool {
    // Check every component in the path.
    for component in path.components() {
        if let Some(name) = component.as_os_str().to_str() {
            // Skip hidden folders (names starting with a dot)
            if name.starts_with('.') {
                return true;
            }
            // If any component matches one of our skip folder names, skip the folder.
            if SKIP_FOLDERS.contains(&name) {
                return true;
            }
        }
    }
    false
}

/// Determine if file should be skipped (binary, large, or user ignored).
fn should_skip_file(path: &Path, user_ignores: &[String], debug: bool) -> bool {
    if path.is_dir() {
        return true;
    }

    // Check extension
    let ext = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_lowercase();
    if !RECOGNIZED_EXTENSIONS.contains(&ext.as_str()) {
        // Possibly a known binary or else unrecognized
        if BINARY_FILE_EXTENSIONS.contains(&ext.as_str()) {
            if debug {
                eprintln!("Skipping known-binary file: {}", path.display());
            }
            return true;
        }
        if debug {
            eprintln!("Skipping unrecognized extension: {}", path.display());
        }
        return true;
    }

    // Check user ignore
    let pstr = path.to_string_lossy().to_string();
    for pat in user_ignores {
        if pstr.contains(pat) {
            if debug {
                eprintln!("Skipping file by user ignore pattern: {}", path.display());
            }
            return true;
        }
    }

    // Check size
    if let Ok(md) = path.metadata() {
        if md.len() > DEFAULT_MAX_FILE_SIZE {
            if debug {
                eprintln!("Skipping large file: {} (>5MB)", path.display());
            }
            return true;
        }
    }

    false
}

fn is_excluded_path(path: &Path, excludes: &[PathBuf]) -> bool {
    // We’ll do a canonicalize on the `path` so that comparisons are consistent:
    let path_canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false, // If we can't canonicalize, skip trying to exclude
    };

    for exc in excludes {
        // canonicalize each exclude as well (you might do it once ahead of time)
        if let Ok(exc_canon) = exc.canonicalize() {
            // If path is inside exc_canon, i.e. path starts with exc_canon
            if path_canonical.starts_with(&exc_canon) {
                return true;
            }
        }
    }
    false
}

fn collect_files(
    dir: &Path,
    user_ignores: &[String],
    excludes: &[PathBuf],
    debug: bool,
) -> Result<Vec<FileEntry>, Box<dyn Error>> {
    let mut entries = Vec::new();

    if !dir.is_dir() {
        return Ok(entries);
    }

    let walker = WalkBuilder::new(dir)
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .build();

    for entry in walker {
        let ent = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = ent.path();

        // First, check if this path or any parent is excluded
        if is_excluded_path(path, excludes) {
            if debug {
                eprintln!("Skipping excluded path: {}", path.display());
            }
            continue;
        }

        // Then, if it's a folder, see if it's in the skip list
        if path.is_dir() && should_skip_folder(path) {
            continue;
        }
        // Next, skip the file if it fails our "should_skip_file" logic
        if should_skip_file(path, user_ignores, debug) {
            continue;
        }

        // Attempt reading the file contents
        match fs::read_to_string(path) {
            Ok(content) => {
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                // Parse the file into code chunks.
                // (These may be multiple if the parser splits the file.)
                let code_chunks = parse::parse_file_to_chunks(&content, &ext);

                // Join all chunks into a single continuous string.
                let joined_content = code_chunks
                    .into_iter()
                    .map(|chunk| chunk.text)
                    .collect::<String>();

                let rel_path = make_relative(dir, path);
                entries.push(FileEntry {
                    rel_path, // No "(part)" suffixes anymore.
                    content: joined_content,
                });
            }
            Err(e) => {
                if debug {
                    eprintln!("Skipping unreadable file {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(entries)
}

/// Convert path->string relative to `base`, always using forward slashes
fn make_relative(base: &Path, target: &Path) -> String {
    match target.strip_prefix(base) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => target.to_string_lossy().replace('\\', "/"),
    }
}
