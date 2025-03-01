# Diff

A simple command-line tool for computing diffs between strings, files, or program outputs. The diffs are displayed in color to make them easier to read.

---

## Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/Eeeeast/diff
   cd diff
   ```

2. Build the project:

   ```bash
   cargo build --release
   ```

3. Run the binary:

   ```bash
   ./target/release/diff --help
   ```

---

## Usage

### Basic Commands

- **Get**: Compute the diff between two inputs.
- **Example**: Generate example test cases in TOML format.

Run `diff --help` for detailed options.

---

## Modes

### Interactive Mode

Compare two directly entered strings. The differences are highlighted in color.

```bash
diff get string1 string2 -m interactive
```

### Batch Mode

Compare the contents of two files. Differences are shown with colored highlights.

```bash
diff get file1.txt file2.txt -m batch
```

### Program Mode

Run a program with test cases and compare its output. Differences between expected and actual outputs are displayed in color.

```bash
diff get my_program tests.toml -m program
```

---

## Generating Test Cases

Generate example test cases in TOML format.

```bash
diff example --count 5 --path tests.toml
```

If no path is specified, the test cases will be printed to the console.

---

## Example

Compare two strings:

```bash
diff get hello world -m interactive
```

Output (with colors):
```
hell[w]o[rld]
```

- **Red**: Deleted content (`hell`)
- **Cyan**: Inserted content (`[w]` and `[rld]`)
- **Default**: Unchanged content (`o`)

---

### Notes on Colored Output

- **Deleted Content**: Highlighted in **red**.
- **Inserted Content**: Highlighted in **cyan**.
- **Unchanged Content**: Displayed in the default text color.

This makes it easy to visually identify differences between the compared inputs.
