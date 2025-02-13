use atty; // for checking if stdout is a TTY
use clap::{Arg, ArgAction, Command};
use ignore::WalkBuilder;
use rayon::prelude::*;
use serde::Deserialize;
use serde_yaml;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir; // NEW: for parallel processing // NEW: For in-memory ZIP reading

// NEW: For downloading repositories and unzipping
use reqwest;
use zip::ZipArchive;

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
        .version("0.0.9")
        .author("Stanislav Kirdey")
        .about("r2md: merges code from multiple directories, streams or writes Markdown, and can optionally produce PDF.")
        .arg(
            Arg::new("paths")
                .help("One or more directories or git repo URLs to process")
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
            Arg::new("include")
                .long("include")
                .help("Include only files matching the given pattern (supports glob patterns, e.g., *.tf)")
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

    let includes: Vec<String> = matches
        .get_many::<String>("include")
        .unwrap_or_default()
        .map(|s| s.to_string())
        .collect();
    

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
    for input in &directories {
        let input_str = input.to_string_lossy();
        if input_str.starts_with("http://") || input_str.starts_with("https://") {
            let git_files = collect_files_from_git_url(&input_str, &user_ignores, &includes, debug_mode)?;
            all_files.extend(git_files);
        } else {
            let collected = collect_files_parallel(input, &user_ignores, &excludes, &includes, debug_mode)?;
            all_files.extend(collected);
        }
    }
    

    if streaming {
        stream_markdown(&all_files)?;
        return Ok(());
    }

    // Build the Markdown output with proper code fences.
    let mut md_output = String::new();
    for dir in &directories {
        md_output.push_str("```\n");
        md_output.push_str(&generate_directory_tree(
            dir,
            &user_ignores,
            &includes,
            debug_mode
        )?);
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

fn collect_files_from_git_url(
    url: &str,
    user_ignores: &[String],
    includes: &[String],
    debug: bool,
) -> Result<Vec<FileEntry>, Box<dyn Error>> {
    // Remove trailing ".git" if present.
    let mut base_url = url.to_string();
    if base_url.ends_with(".git") {
        base_url = base_url.trim_end_matches(".git").to_string();
    }

    // Closure that attempts to download the ZIP archive for a given branch.
    let try_download = |branch: &str| -> Result<reqwest::blocking::Response, Box<dyn Error>> {
        let download_url = if base_url.ends_with('/') {
            format!("{}archive/refs/heads/{}.zip", base_url, branch)
        } else {
            format!("{}/archive/refs/heads/{}.zip", base_url, branch)
        };
        if debug {
            eprintln!(
                "Attempting to download repository ZIP from: {}",
                download_url
            );
        }
        let resp = reqwest::blocking::get(&download_url)?;
        if resp.status().is_success() {
            Ok(resp)
        } else {
            Err(format!(
                "Failed to download repository ZIP for branch {}: {}",
                branch,
                resp.status()
            )
            .into())
        }
    };

    // Try the "main" branch first; if that fails, try "master".
    let response = try_download("main").or_else(|err| {
        if debug {
            eprintln!("Main branch download failed: {}", err);
        }
        try_download("master")
    })?;

    // Continue as before: read the ZIP archive from memory.
    let bytes = response.bytes()?;
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader)?;

    let mut file_entries = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.is_dir() {
            continue;
        }
        let full_name = file.name();
        let path = Path::new(full_name);
        let mut components = path.components();
        let _ = components.next(); // skip top-level folder
        let rel_path = components.as_path().to_string_lossy().to_string();

        if !includes.is_empty() {
            let normalized_path = rel_path.replace('\\', "/");
            let matches_include = includes.iter().any(|pattern| {
                glob::Pattern::new(pattern)
                    .map(|p| p.matches(&normalized_path))
                    .unwrap_or(false)
            });
            
            if matches_include {
                // Bypass all checks for included files
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    file_entries.push(FileEntry { rel_path, content });
                }
                continue;
            }
        }

        // (Continue with existing size, extension, and user ignore checks.)
        if file.size() > DEFAULT_MAX_FILE_SIZE {
            if debug {
                eprintln!("Skipping large file from zip: {}", rel_path);
            }
            continue;
        }

        let ext = Path::new(&rel_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !RECOGNIZED_EXTENSIONS.contains(&ext.as_str()) {
            if BINARY_FILE_EXTENSIONS.contains(&ext.as_str()) {
                if debug {
                    eprintln!("Skipping known binary file from zip: {}", rel_path);
                }
                continue;
            }
            if debug {
                eprintln!("Skipping unrecognized extension file from zip: {}", rel_path);
            }
            continue;
        }

        if user_ignores.iter().any(|pat| rel_path.contains(pat)) {
            if debug {
                eprintln!("Skipping file by user ignore pattern from zip: {}", rel_path);
            }
            continue;
        }

        let mut content = String::new();
        if let Err(e) = file.read_to_string(&mut content) {
            if debug {
                eprintln!("Skipping unreadable file {}: {}", rel_path, e);
            }
            continue;
        }

        file_entries.push(FileEntry { rel_path, content });
    }
    Ok(file_entries)    
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

fn generate_directory_tree(dir: &Path, user_ignores: &[String], includes: &[String], debug: bool) -> Result<String, Box<dyn Error>> {
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

        if should_skip_folder(path) {
            continue;
        }

        // Use your real variables: user_ignores, includes, debug
        if !path.is_dir() && should_skip_file(path, user_ignores, includes, debug) {
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

fn should_skip_file(
    path: &Path,
    user_ignores: &[String],
    includes: &[String],  // <-- add includes
    debug: bool,
) -> bool {
    // (1) If the file matches an `--include` pattern, do NOT skip it.
    if !includes.is_empty() {
        let file_str = path.to_string_lossy();
        let matches_include = includes.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(&file_str))
                .unwrap_or(false)
        });
        if matches_include {
            if debug {
                eprintln!("File {} matches include => not skipping extension checks", path.display());
            }
            return false; // file is explicitly included, so do NOT skip
        }
    }

    // (2) Otherwise, do your usual extension, binary, size, etc. checks...
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

    // user ignore check
    let pstr = path.to_string_lossy().to_string();
    for pat in user_ignores {
        if pstr.contains(pat) {
            if debug {
                eprintln!("Skipping file by user ignore pattern: {}", path.display());
            }
            return true;
        }
    }

    // size check
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

fn collect_files_parallel(
    dir: &Path,
    user_ignores: &[String],
    excludes: &[PathBuf],
    includes: &[String],
    debug: bool,
) -> Result<Vec<FileEntry>, Box<dyn Error>> {
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let walker = WalkBuilder::new(dir)
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .build();

    let paths: Vec<PathBuf> = walker
        .filter_map(|entry| match entry {
            Ok(ent) => {
                let path = ent.path();

                // Check for force-inclusion via --include first
                let mut force_include = false;
                if !includes.is_empty() {
                    let rel_path = match path.strip_prefix(dir) {
                        Ok(p) => p.to_string_lossy().replace('\\', "/"),
                        Err(_) => path.to_string_lossy().replace('\\', "/"),
                    };
                    
                    force_include = includes.iter().any(|pattern| {
                        glob::Pattern::new(pattern)
                            .map(|p| p.matches(&rel_path))
                            .unwrap_or(false)
                    });
                }

                 // Force include matches immediately
                if force_include {
                    return Some(path.to_path_buf());
                }

                // 2) Then your usual exclude logic
                if is_excluded_path(path, excludes) {
                    if debug {
                        eprintln!("Skipping excluded path: {}", path.display());
                    }
                    return None;
                }
                if path.is_dir() && should_skip_folder(path) {
                    return None;
                }
                if !path.is_dir() && should_skip_file(path, user_ignores, includes, debug) {
                    return None;
                }

                Some(path.to_path_buf())
            }
            Err(_) => None,
        })
        .collect();

    // Finally, read & parse the remaining files
    let file_entries: Vec<FileEntry> = paths
        .par_iter()
        .filter_map(|path| match fs::read_to_string(path) {
            Ok(content) => {
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let code_chunks = parse::parse_file_to_chunks(&content, &ext);
                let joined_content = code_chunks.into_iter()
                    .map(|chunk| chunk.text)
                    .collect::<String>();

                Some(FileEntry {
                    rel_path: make_relative(dir, path),
                    content: joined_content,
                })
            }
            Err(e) => {
                if debug {
                    eprintln!("Skipping unreadable file {}: {}", path.display(), e);
                }
                None
            }
        })
        .collect();

    Ok(file_entries)
}



/// Convert path->string relative to `base`, always using forward slashes
fn make_relative(base: &Path, target: &Path) -> String {
    match target.strip_prefix(base) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => target.to_string_lossy().replace('\\', "/"),
    }
}



#[test]
fn test_path_utilities() {
    assert_eq!(
        make_relative(Path::new("/base"), Path::new("/base/file.txt")),
        "file.txt"
    );
    assert_eq!(
        make_relative(Path::new("/base"), Path::new("/other/file.txt")),
        "/other/file.txt"
    );
}

#[test]
fn test_pdf_generation() -> Result<(), Box<dyn std::error::Error>> {
    let files = vec![FileEntry {
        rel_path: "test.rs".into(),
        content: "fn main() {}".into(),
    }];
    
    let temp_file = tempfile::NamedTempFile::new()?;
    let path = temp_file.path().to_str().unwrap();
    
    write_pdf_file(&files, &[PathBuf::from(".")], path)?;
    assert!(Path::new(path).exists());
    
    Ok(())
}