"""Download book sources from GitHub and generate SUMMARY.md and meta.json."""

import json
import logging
import tomllib
from collections import deque
from datetime import UTC, date, datetime
from pathlib import Path
from typing import Literal

import httpx

src_dir = Path(__file__).parent / "src"
logger = logging.getLogger(__name__)

client = httpx.Client(timeout=30.0)

summary: deque[str] = deque(
    [
        "# Summary",
        "[Introduction](./index.md)",
        "",  # An empty line is required after prefix chapters
    ]
)
"""Lines in SUMMARY.md."""

legal_dir = src_dir / "license"
legal_files: deque[tuple[str, Path]] = deque()
"""Legal files `[(src_url, dst_path)]` to be downloaded in the end."""

meta_map: dict[str, str] = {}
"""A mapping from file paths to source URLs."""


def download(src_url: str, dst_path: Path, /, *, title: str, level: Literal[0, 1] = 1) -> None:
    """Download a file from a given URL (if not already downloaded), and append the entry to SUMMARY.md."""
    dst_short = dst_path.relative_to(src_dir).as_posix()
    summary.append(f"{'  ' * level}- [{title}](./{dst_short})")

    assert dst_short not in meta_map, f"Duplicate entry for {dst_short}"
    meta_map[dst_short] = src_url.replace("/raw/", "/blob/")

    if dst_path.exists():
        logger.info(f"Skip downloading {dst_short}, already exists")
    else:
        logger.info(f"Downloading {dst_short}")

        dst_path.parent.mkdir(parents=True, exist_ok=True)

        r = client.get(src_url, follow_redirects=True)
        r.raise_for_status()
        dst_path.write_text(normalize_markdown(r.text), encoding="utf-8")


def normalize_markdown(content: str) -> str:
    """Ad hoc normalization for https://docs.rs/pulldown-cmark-to-cmark"""
    # For https://github.com/typst/packages/blob/09e558d2b8a5342dc6c273e6ec85eb2da1c47b44/docs/manifest.md?plain=1#L195-L196
    # pulldown-cmark can recognize it, but pulldown-cmark-to-cmark will collapse `[local packages]: …` to `[localpackages]: …` without this normalization.
    return content.replace("[local\n  packages]", "[local packages]")


def download_typst(repo, /) -> None:
    typst = src_dir / "typst"

    download(f"{repo}/README.md", typst / "index.md", title="Typst", level=0)
    download(f"{repo}/docs/dev/architecture.md", typst / "dev/architecture.md", title="Compiler architecture")

    for f in ["LICENSE", "NOTICE"]:
        legal_files.append((f"{repo}/{f}", legal_dir / "typst" / f))


def download_codex(repo, /) -> None:
    codex = src_dir / "codex"

    download(f"{repo}/README.md", codex / "index.md", title="Codex", level=0)
    download(f"{repo}/CONTRIBUTING.md", codex / "contributing.md", title="Contributing guidelines")
    download(f"{repo}/CHANGELOG.md", codex / "changelog.md", title="Changelog")

    for f in ["LICENSE"]:
        legal_files.append((f"{repo}/{f}", legal_dir / "codex" / f))


def download_hayagriva(repo, /) -> None:
    hayagriva = src_dir / "hayagriva"

    download(f"{repo}/README.md", hayagriva / "index.md", title="Hayagriva", level=0)

    download(f"{repo}/docs/file-format.md", hayagriva / "file-format.md", title="YAML format")
    download(f"{repo}/docs/selectors.md", hayagriva / "selectors.md", title="Bibliography selectors")

    download(f"{repo}/CHANGELOG.md", hayagriva / "changelog.md", title="Changelog")

    for f in ["LICENSE-MIT", "LICENSE-APACHE", "NOTICE"]:
        legal_files.append((f"{repo}/{f}", legal_dir / "hayagriva" / f))


def download_packages(repo, /) -> None:
    packages = src_dir / "packages"

    download(f"{repo}/README.md", packages / "index.md", title="Packages", level=0)
    download(f"{repo}/docs/README.md", packages / "submission.md", title="Submission guidelines")

    for file, title in [
        ("manifest.md", "Package manifest"),
        ("typst.md", "Typst files"),
        ("resources.md", "Images, fonts and other assets"),
        ("documentation.md", "The README file, and documentation in general"),
        ("licensing.md", "Licensing your package"),
        ("tips.md", "Further tips"),
        ("CATEGORIES.md", "List of categories"),
        ("DISCIPLINES.md", "List of disciplines"),
    ]:
        download(f"{repo}/docs/{file}", packages / file.lower(), title=title)

    for f in ["LICENSE"]:
        legal_files.append((f"{repo}/{f}", legal_dir / "packages" / f))


def load_versions() -> dict[Literal["typst", "codex", "hayagriva", "packages"], tuple[date, str]]:
    """Load `repo_name => (author_date, repo_url_base)` from book.toml."""

    book_toml = src_dir.parent / "book.toml"
    with book_toml.open("rb") as f:
        config = tomllib.load(f)["preprocessor"]["typst-extra-docs"]["download"]

    result = {}

    for k, v in config.items():
        commit_hash, author_date_str = v.split(" ", 1)
        author_date = datetime.fromisoformat(author_date_str).astimezone(UTC).date()
        repo_url_base = f"https://github.com/typst/{k}/raw/{commit_hash}"
        result[k] = (author_date, repo_url_base)

    return result


if __name__ == "__main__":
    logging.basicConfig(
        level=logging.INFO,
        format="\033[1;32m%(asctime)s\033[0m - \033[1;34m%(levelname)s\033[0m - %(message)s",
    )

    versions = load_versions()

    (src_dir / "index.md").write_text(
        (src_dir.parent / "README.md").read_text(encoding="utf-8").replace("](./src/", "](./"),
        encoding="utf-8",
    )

    download_typst(versions["typst"][1])
    summary.append("  - [Snapshots of docs](./zim/index.md)")
    download_codex(versions["codex"][1])
    download_hayagriva(versions["hayagriva"][1])
    download_packages(versions["packages"][1])

    summary.append("- [Licenses](./licenses.md)")
    for src_url, dst_path in legal_files:
        download(src_url, dst_path.with_suffix(".md"), title=f"{dst_path.parent.name.title()}: {dst_path.stem}")

    (src_dir / "SUMMARY.md").write_text("\n".join(summary) + "\n", encoding="utf-8")
    (src_dir / "meta.json").write_text(
        json.dumps(
            {
                "dates": {
                    repo.replace("/raw/", "/blob/"): author_date.isoformat()
                    for (author_date, repo) in versions.values()
                },
                "map": meta_map,
            },
            ensure_ascii=False,
        ),
        encoding="utf-8",
    )
