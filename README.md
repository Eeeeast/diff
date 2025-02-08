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
diff get --left "string1" --right "string2" --mode interactive
```

### Batch Mode

Compare the contents of two files. Differences are shown with colored highlights.

```bash
diff get --left file1.txt --right file2.txt --mode batch
```

### Program Mode

Run a program with test cases and compare its output. Differences between expected and actual outputs are displayed in color.

```bash
diff get --left my_program --right tests.toml --mode program
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
diff get --left "hello" --right "world" --mode interactive
```

Output (with colors):
```
h[ello]w[orld]
```

- **Red**: Deleted content (`[ello]`)
- **Cyan**: Inserted content (`[orld]`)
- **Default**: Unchanged content (`h` and `w`)

---

## License

This project is licensed under the MIT License.

---

## Contact

For questions or feedback, open an issue on the GitHub repository or email [your-email@example.com](mailto:your-email@example.com).

---

### Notes on Colored Output

- **Deleted Content**: Highlighted in **red**.
- **Inserted Content**: Highlighted in **cyan**.
- **Unchanged Content**: Displayed in the default text color.

This makes it easy to visually identify differences between the compared inputs.
