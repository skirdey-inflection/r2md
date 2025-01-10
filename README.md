# r2md

A Rust-based command-line tool that scans a local repository, extracts **just the code** files (for ~20 popular languages), and produces a single **Markdown** file.
It helps you ignore common dependency folders (e.g., `node_modules`, `venv`, `target`) and dotfiles while preserving the essential directory structure and file contents. Useful for quickly sharing or analyzing a project in a single file, or for feeding that file to Large Language Models (LLMs).

## What Problem Does It Solve?

1. **Single-File Snapshot**: Gather all code (excluding dependencies) in one Markdown for easy review or sharing.  
2. **Automated Filtering**: Respects `.gitignore`, common hidden folders, and typical dependency directories to **avoid clutter**.  
3. **Cross-Language Support**: Recognizes ~20 popular programming languages (by file extension) and ignores the rest.  
4. **Convenient Output**: Generate Markdown for easy reading, without manual or complicated steps.

## How to Compile

1. **Install Rust** (with [rustup](https://rustup.rs/)) if you haven’t already:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. **Clone or download** this repository (or place the two main files, `Cargo.toml` and `src/main.rs`, in a new directory).
3. In that directory, run:
   ```bash
   cargo build --release
   ```
   This creates a release build in `target/release/r2md` (on macOS/Linux) or `target\release\r2md.exe` (on Windows).

## How to Run

- **Basic usage** (scan the current directory, output to `r2md_output.md`):
  ```bash
  cargo run
  ```
  or, if you prefer the release binary:
  ```bash
  ./target/release/r2md
  ```
  (on Windows: `.\target\release\r2md.exe`).

- **Specifying a repo path** and custom output:
  ```bash
  ./r2md /path/to/your/repository -o myoutput.md
  ```
  This scans the specified directory, ignoring non-code files, and writes the result to `myoutput.md`.

- **Include a PDF** using the `-p` or `--pdf` flag:
  ```bash
  ./r2md . --pdf
  ```
  This generates both `r2md_output.md` and `r2md_output.pdf`.  
  Or if you specify an output name, e.g. `-o myexport.md`, a `myexport.pdf` will also be produced.

### Command-Line Arguments

```
Usage: r2md [PATH] [OPTIONS]

Arguments:
  [PATH]    Path to the repository (default: current directory)

Options:
  -o, --output <FILE>     Output Markdown file name (default: r2md_output.md)
  -p, --pdf               Also produce a PDF file (default: r2md_output.pdf if no output is specified)
  -h, --help              Print help information
  -V, --version           Print version information
```

## How to Install System-Wide (macOS, Linux, Windows)

### macOS/Linux

1. **Build** the release version:
   ```bash
   cargo build --release
   ```
2. Copy the binary (`r2md`) to a directory in your `$PATH`, for example:
   ```bash
   sudo cp target/release/r2md /usr/local/bin/
   ```
3. Now you can run:
   ```bash
   r2md --help
   ```

### Windows

1. **Build** the release version:
   ```powershell
   cargo build --release
   ```
2. You will get an executable at `target\release\r2md.exe`.
3. Copy or move this `.exe` to a folder in your `PATH`. For example:
   - Create a folder `C:\bin` if it doesn’t exist.
   - Copy:
     ```powershell
     copy .\target\release\r2md.exe C:\bin\
     ```
   - Add `C:\bin` to your [Windows system `PATH`](https://java.com/en/download/help/path.xml) if it isn’t there already.
4. You can now run:
   ```powershell
   r2md --help
   ```
   from any folder.