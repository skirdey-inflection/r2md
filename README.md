```markdown
# r2md: Repository to Markdown

`r2md` is a command-line tool that generates Markdown documentation from your code repository. It parses your project's directory structure and includes code snippets from recognized files, making it easy to create comprehensive overviews of your codebase.

## Features

*   **Directory Structure:** Generates a clear, hierarchical representation of your repository's directory structure in Markdown.
*   **Code Inclusion:** Automatically includes the content of recognized code files (e.g., `.rs`, `.py`) in Markdown code blocks.
*   **Language-Specific Parsing:**
    *   Uses [tree-sitter](https://tree-sitter.github.io/) for more intelligent parsing of **Python** and **Rust** code, extracting functions, classes, structs, enums, and more as individual code blocks.
    *   Provides a line-based fallback for other file types, attempting to identify logical code blocks.
*   **Output Options:**
    *   **Markdown File Output:** Writes the generated documentation to a Markdown file (default: `r2md_output.md`).
    *   **Streaming to STDOUT:**  Outputs the Markdown directly to standard output when the output is piped.
    *   **PDF Generation (Optional):**  Can optionally produce a PDF version of the documentation using the `-p` or `--pdf` flag.
*   **Configuration:** Allows customization through an optional `r2md.yml` or `r2md.yaml` configuration file to specify ignore patterns for files and directories.
*   **Ignore Patterns:** Supports ignoring specific files or directories based on substring matching defined in the configuration file.
*   **File Size Limits:** Skips processing of large files (default: 5MB) to improve performance.
*   **Debug Mode:** Provides a `--debug` flag for verbose output, showing which files are being skipped and why.
*   **Training Data Generation:**  Can generate JSON training data (`--train-json`) containing prompt and completion pairs extracted from your code, useful for fine-tuning language models. It utilizes the `cl100k_base` tokenizer (used by GPT-4).

## Installation

Make sure you have Rust installed. You can install `r2md` using `cargo`:

```bash
cargo install r2md
```

## Usage

Basic usage involves pointing `r2md` to one or more directories you want to document:

```bash
r2md path/to/your/code
```

This will generate a `r2md_output.md` file in the current directory.

**Options:**

*   **Specify Output File:** Use the `-o` or `--output` flag to specify the output Markdown file name:

    ```bash
    r2md path/to/your/code -o my_documentation.md
    ```

*   **Generate PDF:** Use the `-p` or `--pdf` flag to also generate a PDF file:

    ```bash
    r2md path/to/your/code -p
    ```
    or
    ```bash
    r2md path/to/your/code -o my_documentation.md --pdf
    ```
    The PDF file will have the same name as the Markdown file, with the `.pdf` extension.

*   **Enable Debug Mode:** Use the `--debug` flag for more verbose output:

    ```bash
    r2md path/to/your/code --debug
    ```

*   **Generate LLM Training Data:** Use the `--train-json` flag to generate a JSON file containing training data:

    ```bash
    r2md path/to/your/code --train-json training_data.json
    ```

*   **Multiple Directories:** Provide multiple directory paths to process them all in one go:

    ```bash
    r2md path/to/dir1 path/to/dir2
    ```

*   **Streaming Output:** Pipe the output to another command or file to stream the Markdown:

    ```bash
    r2md path/to/your/code | less
    ```

## Configuration

You can configure `r2md` using an optional `r2md.yml` or `r2md.yaml` file in the directory where you run the command. This file allows you to specify patterns for files and directories that should be ignored.

**Example `r2md.yml`:**

```yaml
ignore_patterns:
  - ".git"
  - "target"
  - "node_modules"
  - "_old.rs"
```

Files or directories with names containing any of these patterns will be skipped during the documentation generation process.

## Code Parsing Details

`r2md` leverages the power of [tree-sitter](https://tree-sitter.github.io/) for parsing code. This allows for a more structured and intelligent extraction of code blocks, particularly for:

*   **Python:** Extracts function definitions (`def`), class definitions (`class`).
*   **Rust:** Extracts function items (`fn`), struct items (`struct`), enum items (`enum`), impl items (`impl`), and trait items (`trait`).

For other file extensions, `r2md` falls back to a simpler line-based parsing approach, attempting to identify logical code blocks based on common keywords like `function`, `class`, or `def`.

## Training Data Generation

The `--train-json` feature generates training samples by splitting the content of each recognized code file into a "prompt" and a "completion". It uses an 80/20 split, with the first 80% of the tokens as the prompt and the remaining 20% as the completion. This data can be used for training or fine-tuning language models. The generated JSON includes:

*   `prompt`: The initial part of the code.
*   `completion`: The subsequent part of the code.
*   `prompt_tokens`: The number of tokens in the prompt.
*   `completion_tokens`: The number of tokens in the completion.
*   `tokenizer`: The name of the tokenizer used (`cl100k_base`).
*   `tokenizer_rs_version`: The version of the tokenizer library.

## Example Output

Here's an example of the kind of output `r2md` produces:

```markdown
# Repository Markdown Export

## Directory Structure

```
- my_project/
  - src/
    - main.rs
    - utils.rs
  - examples/
    - example.rs
```

## Code

### `src/main.rs`

```plaintext
fn main() {
    println!("Hello, world!");
}
```

### `src/utils.rs`

```plaintext
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

### `examples/example.rs`

```plaintext
use my_project::greet;

fn main() {
    println!("{}", greet("User"));
}
```

## Contributing

Contributions are welcome! Please feel free to open issues or submit pull requests on the project's repository.