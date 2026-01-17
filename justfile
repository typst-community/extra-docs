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
    uv run download.py

# Build the book
build: download
    mdbook build
    # Now check the book/ directory.

# Serve and open the book
serve *ARGS: download
    mdbook serve {{ ARGS }}
