# r2md - Repository to Markdown

**r2md** is a command-line tool that aggregates code from one or more directories into a single Markdown file. It provides a convenient way to document the structure and contents of your codebase. Optionally, it can also generate a basic PDF representation of the collected code.

## Features

*   **Directory Structure:**  Generates a tree-like representation of the input directories in the Markdown output.
*   **Code Inclusion:** Includes the content of recognized source code files within Markdown code blocks.
*   **File Recognition:**  Recognizes a wide range of programming language files based on their extensions (e.g., `.rs`, `.py`, `.js`, `.java`, etc.).
*   **File Ignoring:**
    *   Skips common binary files (images, executables, archives, etc.).
    *   Ignores files larger than a configurable size (default: 5MB).
    *   Skips common dependency and hidden folders (e.g., `.git`, `target`, `node_modules`).
    *   Supports user-defined ignore patterns via a configuration file (`r2md.yml` or `r2md.yaml`).
*   **Output Options:**
    *   **Markdown File:** Writes the aggregated content to a Markdown file.
    *   **PDF Generation (Optional):** Can also produce a basic PDF version of the code.
    *   **Streaming to STDOUT:** If the output is piped, it streams the Markdown content directly to standard output.
*   **Configuration File:**  Allows customization of ignore patterns through an optional `r2md.yml` or `r2md.yaml` file.
*   **Debug Mode:** Provides verbose output for debugging file inclusion and exclusion.

## Installation

Make sure you have Rust and Cargo installed. You can then install `r2md` using Cargo:

```bash
cargo install r2md
```

## Usage

```bash
r2md [OPTIONS] [PATHS...]
```

### Options

*   `-o, --output <FILE>`:  Specify the output Markdown file name. Defaults to `r2md_output.md` if not streaming to stdout.
*   `-p, --pdf`:  Produce a PDF file in addition to the Markdown file. The PDF will have the same name as the Markdown file but with a `.pdf` extension.
*   `--debug`: Enable debug output, showing which files are being skipped and why.

### Arguments

*   `PATHS...`:  One or more directory paths to process. If no paths are provided, it defaults to the current directory (`.`).

### Examples

**Basic usage, outputting to the default `r2md_output.md` file:**

```bash
r2md
```

**Specify a directory to process:**

```bash
r2md src
```

**Process multiple directories:**

```bash
r2md src examples
```

**Specify the output Markdown file name:**

```bash
r2md -o my_code_documentation.md src
```

**Generate a PDF file as well:**

```bash
r2md -p src
```

**Generate a PDF with a custom output name:**

```bash
r2md -o my_code_documentation.md -p src
```

**Stream the output to stdout (e.g., for piping to other tools):**

```bash
r2md src | less
```

**Using a configuration file to ignore specific patterns:**

```bash
r2md src
```

(Assuming you have an `r2md.yml` or `r2md.yaml` file in the current directory - see Configuration section below).

**Enable debug output:**

```bash
r2md --debug src
```

## Configuration

You can customize the behavior of `r2md` by creating an optional configuration file named `r2md.yml` or `r2md.yaml` in the directory where you run the command.

The configuration file supports the following options:

```yaml
ignore_patterns:
  - "generated/"
  - "tmp_"
  - ".git/"
```

**`ignore_patterns`**: A list of string patterns. If a file path contains any of these patterns, it will be ignored.

## File Recognition and Ignoring Details

**r2md** uses a combination of methods to determine which files to include and exclude:

*   **Recognized Extensions:**  It includes files with extensions commonly associated with source code (e.g., `.rs`, `.py`, `.js`, `.c`, `.cpp`, `.java`, etc.).
*   **Binary File Extensions:** It automatically skips files with extensions known to be binary (e.g., `.jpg`, `.png`, `.exe`, `.dll`, `.pdf`, `.zip`, etc.).
*   **Maximum File Size:** Files larger than 5MB are skipped by default to avoid processing very large files.
*   **Skipped Folders:** Common dependency and hidden folders like `.git`, `target`, `node_modules`, etc., are automatically skipped.
*   **User-Defined Ignores:**  The `ignore_patterns` in the configuration file allow you to specify additional patterns to ignore.

## Output

The generated Markdown file will contain the following sections:

1. **Repository Markdown Export:** A main heading for the document.
2. **Directory Structure:** A section displaying a tree-like representation of the input directories. This helps visualize the organization of your codebase.
3. **Code:** A section containing the content of the recognized source code files. Each file will have a subheading with its relative path, followed by a code block containing the file's content.

The optional PDF output provides a basic rendering of the same information, suitable for viewing or printing the code. Each directory and file is clearly labeled.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests on the project's repository.
