# Contributing guide

## Local preview

First install [mdbook](https://rust-lang.github.io/mdBook/guide/installation.html), [uv](https://docs.astral.sh/uv/#installation), [just](https://just.systems/man/en/), and rust toolchains, then run:

```shell
cargo install --git https://github.com/sicikh/mdbook-typst-highlight --branch upgrade-mdbook-0.5.2
just serve --open
```

It will call `just download` to download book sources from GitHub. If you encounter a network error, usually waiting for a few seconds and then retrying will solve the problem.
