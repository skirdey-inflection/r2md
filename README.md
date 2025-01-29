# r2md

**r2md** is a simple tool that converts your code repositories into a well-structured Markdown file. Whether you want to document your project, create a readable code overview, or prepare content for training machine learning models, r2md makes the process easy and efficient.

## What Problems Does r2md Solve?

- **Code to LLM Made Easy:** See entire codebase as single .md file. 
- **Consolidate Code Snippets:** Merge code from multiple directories into a single, organized Markdown file.
- **Optional PDF Output:** Easily create a PDF version of your codebase for sharing or printing.
- **Training Data Preparation:** Generate JSON files with prompt and completion pairs for SFT training.

## Installation

After downloading the r2md binary, place it in your system's PATH to use it from anywhere in the terminal.

## How to Use r2md

Once the binary is installed, you can use the `r2md` command followed by various options to customize its behavior.

### Basic Command

Convert the current directory into a Markdown file:

```bash
r2md
```

This will generate a `r2md_output.md` file in the current directory.

### Specify Directories

Process one or more specific directories:

```bash
r2md path/to/dir1 path/to/dir2
```

### Exclude Folders

Exclude certain folders from processing:

```bash
r2md -x node_modules -x target
```

### Specify Output File

Define a custom name for the output Markdown file:

```bash
r2md -o my_documentation.md
```

### Generate PDF

Create a PDF version of the Markdown output:

```bash
r2md -p
```

### Enable Debug Mode

Get detailed output for troubleshooting:

```bash
r2md --debug
```

### Generate Training JSON

Create a JSON file with training data using 80 (prompt) /20 (completion) split:

```bash
r2md --train-json training_data.json
```

### Combine Options

Use multiple options together:

```bash
r2md path/to/dir -x .git -o project_docs.md -p --debug --train-json training.json
```

## Example Usage

1. **Generate Markdown from Current Directory:**

   ```bash
   r2md
   ```

   Output: `r2md_output.md`

2. **Generate Markdown and PDF, Excluding `node_modules`:**

   ```bash
   r2md -x node_modules -p
   ```

   Outputs: `r2md_output.md` and `r2md_output.pdf`

3. **Generate Training JSON from Specific Directories:**

   ```bash
   r2md src tests --train-json training_data.json
   ```

   Output: `training_data.json`

## Additional Configuration

r2md can be customized using a YAML configuration file (`r2md.yml` or `r2md.yaml`). This allows you to define additional ignore patterns and other settings.

Example `r2md.yml`:

```yaml
ignore_patterns:
  - "temp"
  - "backup"
```

## Help

For more options and detailed information, use the help flag:

```bash
r2md --help
```
