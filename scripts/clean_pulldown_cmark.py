"""Clean pulldown-cmark side effects.

```markdown
| Table |
|---|
| <pre>parent:<br>  key: ["value"]</pre> |
```

The above markdown displays as expected on GitHub.
However, mdbook will turn `"` into `“”` when smart punctuation is enabled.
https://rust-lang.github.io/mdBook/format/markdown.html#smart-punctuation

Escaping `"` as `&quot;` can avoid the issue.
Unfortunately, pulldown-cmark-to-cmark, the library used by most rust mdbook preprocessors, turns `&quot;` into `"`.
As a result, if such preprocessors are used, escaping the source markdown does not change the markdown received by mdbook.
You can test this with `test_markdown_options` in `typst-extra-docs/src/lib.rs`.

This python script should be run after all other preprocessors. It will turn `“”"` back to `&quot;`.
"""

import json
import re
import sys
from collections.abc import Generator
from typing import Any


def _iter_chapter_impl(items: list[dict[str, Any]]) -> Generator[dict[str, Any]]:
    for item in items:
        if "Chapter" in item:
            chapter = item["Chapter"]
            yield chapter
            yield from _iter_chapter_impl(chapter["sub_items"])


def iter_chapter(book: dict[str, Any]) -> Generator[dict[str, Any]]:
    yield from _iter_chapter_impl(book["items"])


if __name__ == "__main__":
    # Ref: https://rust-lang.github.io/mdBook/for_developers/preprocessors.html#implementing-a-preprocessor-with-a-different-language
    match sys.argv[1:]:
        case ["supports", "html"]:
            sys.exit(0)
        case []:
            pass
        case _:
            print(f"Unsupported arguments: {sys.argv[1:]}", file=sys.stderr)
            sys.exit(1)

    context, book = json.load(sys.stdin)

    for chapter in iter_chapter(book):
        # `cd src; just rg '<pre>'` shows that only this file is affected.
        if chapter["source_path"] == "hayagriva/file-format.md":
            chapter["content"] = "\n".join(
                re.sub(r'[“”"]', "&quot;", line)
                # If this line is a table row containing `<pre>`
                if line.startswith("|") and "<pre>" in line and "</pre>" in line
                else line
                for line in chapter["content"].splitlines()
            )

    print(json.dumps(book, ensure_ascii=False, allow_nan=False))
