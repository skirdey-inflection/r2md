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

/// Config for optional YAML (`r2md.yml` / `r2md.yaml`)
#[derive(Debug, Deserialize)]
struct R2mdConfig {
    /// Additional ignore patterns (substring matches).
    #[serde(default)]
    ignore_patterns: Vec<String>,
}

/// Basic representation of a recognized file
#[derive(Debug)]
struct FileEntry {
    rel_path: String,
    content: String,
}

fn main() -> Result<(), Box<dyn Error>> {
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
        .get_matches();

    // Collect directories from CLI
    let directories: Vec<PathBuf> = matches
        .get_many::<String>("paths")
        .unwrap_or_default()
        .map(PathBuf::from)
        .collect();

    // Check if STDOUT is piped => streaming
    let stdout_is_tty = atty::is(atty::Stream::Stdout);
    let streaming = !stdout_is_tty;

    // Determine output MD file name if not streaming
    let output_md_file = matches
        .get_one::<String>("output")
        .map(|s| s.as_str())
        .unwrap_or("r2md_output.md");

    // Produce a PDF as well?
    let produce_pdf = matches.get_flag("pdf");

    // Load optional YAML config
    let config = load_config_file()?;
    // Gather user ignore patterns
    let mut user_ignores = vec![];
    if let Some(ref c) = config {
        user_ignores.extend(c.ignore_patterns.clone());
    }

    let debug_mode = matches.get_flag("debug");

    // Collect recognized code files from all given directories
    let mut all_files = Vec::new();
    for dir in &directories {
        let collected = collect_files(dir, &user_ignores, debug_mode)?;
        all_files.extend(collected);
    }

    // If streaming -> dump everything to stdout
    if streaming {
        stream_markdown(&all_files)?;
        return Ok(());
    }

    // Otherwise, produce a single .md file
    let mut md_output = String::new();

    // 1) For each directory, generate a directory structure block
    for dir in &directories {
        md_output.push_str("# Repository Markdown Export\n\n");
        md_output.push_str("## Directory Structure\n\n");
        md_output.push_str("```\n");
        md_output.push_str(&generate_directory_tree(dir)?);
        md_output.push_str("```\n\n");
    }

    // 2) Include code listings
    md_output.push_str("## Code\n\n");
    for file in &all_files {
        // Create a heading with the file name
        let heading = format!("### `{}`\n\n", file.rel_path);
        md_output.push_str(&heading);

        // Print file in one code block
        md_output.push_str("```plaintext\n");
        md_output.push_str(&file.content);
        md_output.push_str("\n```\n\n");
    }

    // Write the .md output
    {
        let mut f = BufWriter::new(File::create(output_md_file)?);
        f.write_all(md_output.as_bytes())?;
        f.flush()?;
    }
    println!("Markdown exported to {}", output_md_file);

    // 3) (Optional) Also produce a PDF
    if produce_pdf {
        let pdf_name = if output_md_file == "r2md_output.md" {
            "r2md_output.pdf".to_string()
        } else {
            output_md_file.replace(".md", ".pdf")
        };
        write_pdf_file(&all_files, &directories, &pdf_name)?;
        println!("PDF exported to {}", pdf_name);
    }

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
    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
        if name.starts_with('.') {
            return true;
        }

        if SKIP_FOLDERS.contains(&name) {
            return true;
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

/// Collect recognized files for a single directory
fn collect_files(
    dir: &Path,
    user_ignores: &[String],
    debug: bool,
) -> Result<Vec<FileEntry>, Box<dyn Error>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }

    let walker = WalkBuilder::new(dir)
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .build();

    for entry in walker {
        if let Ok(ent) = entry {
            let p = ent.path();
            if p.is_dir() && should_skip_folder(p) {
                continue;
            }
            if should_skip_file(p, user_ignores, debug) {
                continue;
            }

            match fs::read_to_string(p) {
                Ok(content) => {
                    let rel = make_relative(dir, p);
                    files.push(FileEntry {
                        rel_path: rel,
                        content,
                    });
                }
                Err(e) => {
                    if debug {
                        eprintln!("Skipping unreadable file {}: {}", p.display(), e);
                    }
                }
            }
        }
    }
    Ok(files)
}

/// Convert path->string relative to `base`, always using forward slashes
fn make_relative(base: &Path, target: &Path) -> String {
    match target.strip_prefix(base) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => target.to_string_lossy().replace('\\', "/"),
    }
}

/// Stream output to stdout (no chunking, entire file in one code block)
fn stream_markdown(files: &[FileEntry]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    writeln!(handle, "# r2md Streaming Output\n")?;

    for file in files {
        writeln!(handle, "### `{}`\n", file.rel_path)?;
        writeln!(handle, "```")?;
        writeln!(handle, "{}", file.content)?;
        writeln!(handle, "```")?;
        writeln!(handle)?;
    }

    handle.flush()
}

/// Produce a PDF with a simple text layout
fn write_pdf_file(
    files: &[FileEntry],
    directories: &[PathBuf],
    output_file_name: &str,
) -> Result<(), Box<dyn Error>> {
    let (doc, page1, layer1) = PdfDocument::new("r2md PDF", Mm(210.0), Mm(297.0), "Layer 1");
    let font = doc.add_builtin_font(BuiltinFont::Courier)?;
    let mut current_layer = doc.get_page(page1).get_layer(layer1);

    let mut current_y = 270.0;

    // Print a header for each directory
    for d in directories {
        if current_y < 20.0 {
            let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Layer next");
            current_layer = doc.get_page(p).get_layer(l);
            current_y = 270.0;
        }
        let text = format!("Directory: {}\n", d.display());
        current_layer.use_text(text, 12.0, Mm(10.0), Mm(current_y), &font);
        current_y -= 10.0;
    }

    // Then each file
    for file in files {
        if current_y < 20.0 {
            let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Layer next");
            current_layer = doc.get_page(p).get_layer(l);
            current_y = 270.0;
        }
        let heading = format!("File: {}\n", file.rel_path);
        current_layer.use_text(heading, 10.0, Mm(10.0), Mm(current_y), &font);
        current_y -= 6.0;

        // Print the file content line by line
        for line in file.content.lines() {
            if current_y < 10.0 {
                let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Layer next");
                current_layer = doc.get_page(p).get_layer(l);
                current_y = 270.0;
            }
            current_layer.use_text(line, 8.0, Mm(10.0), Mm(current_y), &font);
            current_y -= 4.0;
        }
        current_y -= 4.0; // extra gap
    }

    doc.save(&mut BufWriter::new(File::create(output_file_name)?))?;
    Ok(())
}
