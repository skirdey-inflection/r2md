use clap::{Arg, ArgAction, Command};
use ignore::{DirEntry, WalkBuilder};
use walkdir::WalkDir;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

// PDF-related imports
use printpdf::{
    Mm, PdfDocument, BuiltinFont,
    // Removed the unused ones: IndirectFontRef, PdfDocumentReference, PdfPageReference,
//  PdfLayerIndex, PdfLayerReference,
};

/// For demonstration, here's a simple struct to hold a recognized file's content.
#[derive(Debug)]
struct FileEntry {
    rel_path: String, // e.g. "src/main.rs"
    content: String,  // file's text
}

/// For simplicity, define a set of file extensions for top ~20 languages.
/// (Feel free to add or remove as needed.)
static RECOGNIZED_EXTENSIONS: &[&str] = &[
    // 1. Rust
    "rs",
    // 2. Python
    "py",
    // 3. JavaScript
    "js",
    // 4. TypeScript
    "ts",
    // 5. C
    "c",
    "h",
    // 6. C++
    "cpp",
    "hpp",
    "cc",
    "cxx",
    "hh",
    // 7. Java
    "java",
    // 8. C#
    "cs",
    // 9. Go
    "go",
    // 10. Ruby
    "rb",
    // 11. PHP
    "php",
    // 12. Swift
    "swift",
    // 13. Kotlin
    "kt",
    "kts",
    // 14. Objective-C
    "m",
    // 15. Objective-C++
    "mm",
    // 16. Shell scripts
    "sh",
    // 17. Batch
    "bat",
    // 18. F#
    "fs",
    // 19. Visual Basic
    "vb",
    // 20. Scala (if you wish)
    "scala",
];

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments
    let matches = Command::new("repo2markdown")
        .version("0.4.0")
        .author("Your Name <you@example.com>")
        .about("Converts a local GitHub repository into a single Markdown file or PDF.")
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
                .help("Output Markdown file name (default: r2md_output.md)")
                .required(false),
        )
        .arg(
            Arg::new("pdf")
                .short('p')
                .long("pdf")
                .help("Also produce a PDF file (default: r2md_output.pdf)")
                // Clap 4: Use action for boolean flags
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let repo_path_str = matches.get_one::<String>("path").unwrap();
    let repo_path = PathBuf::from(repo_path_str);

    let output_md_file = matches
        .get_one::<String>("output")
        .map(|s| s.as_str())
        .unwrap_or("r2md_output.md");

    // Clap 4: Retrieve boolean flags with get_one::<bool> or get_flag
    let produce_pdf = *matches.get_one::<bool>("pdf").unwrap_or(&false);

    if !repo_path.is_dir() {
        eprintln!("Error: provided path is not a directory or doesn't exist.");
        std::process::exit(1);
    }

    // 1) Collect recognized code files
    let files = collect_files(&repo_path)?;

    // 2) Generate Markdown from those files
    let markdown = generate_markdown(&repo_path, &files);

    // 3) Write Markdown to disk
    write_output_file(&markdown, output_md_file)?;

    println!("Markdown exported to {}", output_md_file);

    // 4) If requested, also produce a PDF
    if produce_pdf {
        // Default PDF name is "r2md_output.pdf" if user didn't specify
        // We'll just replace .md with .pdf if the user gave an output name,
        // otherwise use the default "r2md_output.pdf"
        let pdf_name = if output_md_file == "r2md_output.md" {
            "r2md_output.pdf".to_string()
        } else {
            output_md_file.replace(".md", ".pdf")
        };

        write_pdf_file(&files, &repo_path, &pdf_name)?;
        println!("PDF exported to {}", pdf_name);
    }

    Ok(())
}

/// Walk through the repository, respecting .gitignore,
/// but only collecting files that match:
///  - recognized programming language file extension
///  - not hidden (dot-file)
///  - not in a common dependency folder.
fn collect_files(repo_path: &Path) -> Result<Vec<FileEntry>, Box<dyn Error>> {
    let mut files = Vec::new();
    let walker = WalkBuilder::new(repo_path)
        .hidden(false)
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

/// Decide if we should include a file:
///  1. Must be a file (not directory).
///  2. Must not be hidden (leading dot).
///  3. Must not be in known "dependency" folders (venv, node_modules, etc.).
///  4. Must have a recognized extension from our top 20 languages list.
fn should_include(entry: &DirEntry) -> bool {
    let path = entry.path();
    if entry.file_type().map_or(false, |ft| ft.is_dir()) {
        return false;
    }

    let file_name = match path.file_name().and_then(OsStr::to_str) {
        Some(name) => name,
        None => return false,
    };

    // 2) Skip dotfiles (and dot-dirs)
    if file_name.starts_with('.') {
        return false;
    }

    // 3) Skip known dependency directories if they appear in the path
    // (some are repeated from the original, you can expand as needed).
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
                | Some("bin")
                | Some("obj")
                | Some("out")
                | Some("vendor")
        )
    }) {
        return false;
    }

    // 4) Must have a recognized extension
    let ext = match path.extension().and_then(OsStr::to_str) {
        Some(e) => e.to_lowercase(),
        None => return false,
    };
    // Check if the extension is in our recognized list
    if !RECOGNIZED_EXTENSIONS.contains(&ext.as_str()) {
        return false;
    }

    true
}

/// Generate a Markdown string, including:
///  - a high-level directory tree with the root folder name
///  - code blocks for each recognized file
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

/// Generate a directory tree, starting with the top-level folder name,
/// skipping hidden or dependency directories.
fn generate_directory_tree(repo_path: &Path) -> String {
    // 1) Canonicalize the path so "." becomes an absolute path
    let canonical = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());

    // 2) Attempt to get the final component as a folder name
    let root_name = match canonical.file_name().and_then(|s| s.to_str()) {
        Some(fname) => fname.to_string(),
        None => canonical.to_string_lossy().to_string(),
    };

    // Start with root folder line
    let mut output = format!("- {}/\n", root_name);

    // Then walk subdirectories from min_depth(1) so we don't repeat the root
    for entry in WalkDir::new(&canonical).min_depth(1) {
        if let Ok(e) = entry {
            let depth = e.depth();
            let path = e.path();

            // If the path is hidden or in a known dep folder, skip
            if skip_in_tree(path) {
                continue;
            }

            let indent = "  ".repeat(depth);

            let rel_path = path
                .strip_prefix(&canonical)
                .unwrap_or(path)
                .to_string_lossy();

            if e.file_type().is_dir() {
                output.push_str(&format!("{}- {}/\n", indent, rel_path));
            } else {
                output.push_str(&format!("{}- {}\n", indent, rel_path));
            }
        }
    }

    output
}

/// Skip hidden or known dependency directories in the directory tree.
fn skip_in_tree(path: &Path) -> bool {
    for comp in path.components() {
        if let Some(c) = comp.as_os_str().to_str() {
            // If hidden folder/file
            if c.starts_with('.') {
                return true;
            }
            // If known dependency folder, skip
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
                    | "bin"
                    | "obj"
                    | "out"
                    | "vendor"
            ) {
                return true;
            }
        }
    }
    false
}

/// Writes the Markdown to the specified file name on disk.
fn write_output_file(markdown: &str, output_file_name: &str) -> io::Result<()> {
    let file = File::create(output_file_name)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(markdown.as_bytes())?;
    writer.flush()?;
    Ok(())
}

/// A helper function to wrap a single line of text at `width` characters.
/// Returns a `Vec<String>` where each element is a wrapped chunk.
fn wrap_line(line: &str, width: usize) -> Vec<String> {
    let mut wrapped = Vec::new();
    let mut buffer = line.to_string();
    while buffer.chars().count() > width {
        let chunk: String = buffer.chars().take(width).collect();
        let leftover: String = buffer.chars().skip(width).collect();
        wrapped.push(chunk);
        buffer = leftover;
    }
    // Push any remaining text
    if !buffer.is_empty() {
        wrapped.push(buffer);
    }
    wrapped
}

/// (Optional) Generate a simple PDF containing the code of recognized files.
/// This uses `printpdf` in a very basic way (plain text). For advanced formatting,
/// consider a separate Markdown -> PDF tool or something like that.
fn write_pdf_file(
    files: &[FileEntry],
    repo_path: &Path,
    output_file_name: &str,
) -> Result<(), Box<dyn Error>> {
    // Create a new PDF document (A4 size).
    let (doc, page1, layer1) = PdfDocument::new(
        "Repository Code Export", // document title
        Mm(210.0),                // width
        Mm(297.0),                // height
        "Layer 1",                // initial layer name
    );

    // Add a default font (Courier).
    let font = doc.add_builtin_font(BuiltinFont::Courier)?;

    // We'll show the root folder as a header on the first page.
    let mut current_layer = doc.get_page(page1).get_layer(layer1);

    // Print the root folder up top.
    let canonical = repo_path.canonicalize().unwrap_or_else(|_| repo_path.to_path_buf());
    let root_name = canonical
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Unnamed Repository");
    current_layer.use_text(format!("Repository: {}", root_name), 12.0, Mm(10.0), Mm(285.0), &font);

    // We'll keep track of a vertical offset for printing down the page.
    let mut current_y = 270.0; // in mm from bottom

    // We'll do a simple approach: if we run out of space, create a new page.
    let wrap_width = 90;

    for file in files {
        // If we need a new page soon, create it.
        if current_y < 20.0 {
            let (page, layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer next");
            current_layer = doc.get_page(page).get_layer(layer);
            current_y = 270.0;
        }

        // Print the file path
        current_layer.use_text(
            format!("File: {}", file.rel_path),
            10.0,            // font size
            Mm(10.0),        // x position
            Mm(current_y),   // y position
            &font,
        );
        current_y -= 5.0;

        // For each line, wrap if needed, then print
        for line in file.content.lines() {
            // Wrap the line
            let chunks = wrap_line(line, wrap_width);

            for chunk in chunks {
                if current_y < 10.0 {
                    // new page
                    let (page, layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer next");
                    current_layer = doc.get_page(page).get_layer(layer);
                    current_y = 270.0;
                }
                current_layer.use_text(chunk, 8.0, Mm(10.0), Mm(current_y), &font);
                current_y -= 4.0;
            }
        }

        current_y -= 5.0; // gap before next file
    }

    // Save the PDF to disk
    doc.save(&mut BufWriter::new(File::create(output_file_name)?))?;
    Ok(())
}
