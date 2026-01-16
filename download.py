"""Download book sources from GitHub and generate SUMMARY.md and meta.json."""

import json
import logging
from collections import deque
from pathlib import Path
from shutil import copyfile
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
        dst_path.write_text(r.text, encoding="utf-8")


def download_typst(repo, /) -> None:
    typst = src_dir / "typst"

    download(f"{repo}/README.md", typst / "index.md", title="Typst", level=0)
    download(f"{repo}/docs/dev/architecture.md", typst / "dev/architecture.md", title="Compiler architecture")

    for f in ["LICENSE", "NOTICE"]:
        legal_files.append((f"{repo}/{f}", legal_dir / "typst" / f))


def download_codex(repo, /) -> None:
    codex = src_dir / "codex"

    download(f"{repo}/README.md", codex / "index.md", title="Codex", level=0)
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
        ("licensing.md", "Licensing"),
        ("tips.md", "Further tips"),
        ("CATEGORIES.md", "List of categories"),
        ("DISCIPLINES.md", "List of disciplines"),
    ]:
        download(f"{repo}/docs/{file}", packages / file.lower(), title=title)

    for f in ["LICENSE"]:
        legal_files.append((f"{repo}/{f}", legal_dir / "packages" / f))


if __name__ == "__main__":
    logging.basicConfig(
        level=logging.INFO,
        format="\033[1;32m%(asctime)s\033[0m - \033[1;34m%(levelname)s\033[0m - %(message)s",
    )

    copyfile(src_dir.parent / "README.md", src_dir / "index.md")

    download_typst("https://github.com/typst/typst/raw/701c7f9b2853857cde6f4dd76763b9bb118aff14")
    download_codex("https://github.com/typst/codex/raw/cd6d10d732673c27a97b6a42dc1774620a1717cf")
    download_hayagriva("https://github.com/typst/hayagriva/raw/a137441413a5907c15ced44d1502dfb9fa1a3014")
    download_packages("https://github.com/typst/packages/raw/8f21d920ae6389359e4734335a107cca0f57c181")

    summary.append("- [Licenses](./licenses.md)")
    for src_url, dst_path in legal_files:
        download(src_url, dst_path, title=f"{dst_path.parent.name.title()}: {dst_path.stem}")

    (src_dir / "SUMMARY.md").write_text("\n".join(summary) + "\n", encoding="utf-8")
    (src_dir / "meta.json").write_text(json.dumps({"map": meta_map}, ensure_ascii=False), encoding="utf-8")
