manifest := "--manifest-path typst-extra-docs/Cargo.toml"

# List available recipes
list:
    @just --list

# Format files
[group("dev")]
fmt:
    ruff format
    cargo fmt {{ manifest }}

# Check files
[group("dev")]
check:
    ruff check
    cargo clippy {{ manifest }}

# Run tests
[group("dev")]
test:
    cargo test {{ manifest }}

# Run ripgrep
[group("dev")]
[no-cd]
rg PATTERN *ARGS:
    rg --no-ignore-vcs --glob=!meta.json {{ quote(PATTERN) }} {{ ARGS }}

# Download book sources from GitHub (Please rerun if you meet any network error.)
download:
    uv run scripts/download.py

# Build the book
build: download
    mdbook build
    # Now check the book/ directory.

# Serve and open the book
serve *ARGS: download
    mdbook serve {{ ARGS }}

# Check latest version of packages
[group("maintenance")]
check-versions: download
    @{{ if env("GITHUB_TOKEN", "") == "" { error("$GITHUB_TOKEN is required to access GitHub GraphQL API, but it is not set.") } else { "" } }}
    node scripts/check_versions.ts
    # 📝 Now you may update `preprocessor.typst-extra-docs.download` in `book.toml` according to the above output.
    tail --lines 4 book.toml
