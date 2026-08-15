#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "EbookLib>=0.18",
#     "markdown>=3.5",
# ]
# ///
"""Render an OpenSpec change's planning artifacts as a single EPUB.

Collects the markdown artifacts of a change (proposal, capability specs, design,
tasks, plus any extra supporting documents) and assembles them into a navigable
EPUB 3 file with a nested table of contents.

Run it directly -- the PEP 723 header above lets ``uv`` provision dependencies::

    ./scripts/render_change_epub.py                    # auto-detect the active change
    ./scripts/render_change_epub.py add-python-ir-lowering
    ./scripts/render_change_epub.py --list
    ./scripts/render_change_epub.py -o /tmp/report.epub --no-context
"""

from __future__ import annotations

import argparse
import datetime as dt
import re
import subprocess
import sys
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import markdown
from ebooklib import epub

# Artifact ordering: anything not named here sorts after, alphabetically.
ARTIFACT_ORDER: tuple[str, ...] = ("proposal", "design", "tasks")

# Supporting documents pulled from the repo root, in order, when they exist.
CONTEXT_DOCUMENTS: tuple[str, ...] = ("CLAUDE.md", "README.md")

# Words that title-casing would otherwise mangle in generated chapter titles.
ACRONYMS: dict[str, str] = {
    "Ir": "IR",
    "Ast": "AST",
    "Api": "API",
    "Cli": "CLI",
    "Pyo3": "PyO3",
    "Epub": "EPUB",
    "Mvp": "MVP",
}

MARKDOWN_EXTENSIONS: tuple[str, ...] = (
    "extra",  # tables, fenced code, footnotes, def lists
    "sane_lists",
    "toc",  # gives every heading a stable id, used for sub-chapter nav
)

STYLESHEET = """\
body { font-family: serif; line-height: 1.5; margin: 0 5%; }
h1 { font-size: 1.7em; margin: 1.2em 0 0.6em; line-height: 1.25; }
h2 { font-size: 1.3em; margin: 1.4em 0 0.5em; border-bottom: 1px solid #bbb;
     padding-bottom: 0.15em; }
h3 { font-size: 1.1em; margin: 1.2em 0 0.4em; }
h4 { font-size: 1em; margin: 1em 0 0.3em; font-style: italic; }
p { margin: 0.6em 0; }
ul, ol { margin: 0.6em 0; padding-left: 1.4em; }
li { margin: 0.25em 0; }
code { font-family: monospace; font-size: 0.9em; }
pre { font-family: monospace; font-size: 0.85em; background: #f4f4f4;
      padding: 0.7em; overflow-x: auto; border-left: 3px solid #ccc; }
pre code { font-size: 1em; }
blockquote { margin: 0.8em 0 0.8em 1em; padding-left: 0.8em;
             border-left: 3px solid #ccc; color: #444; }
table { border-collapse: collapse; margin: 0.8em 0; }
th, td { border: 1px solid #bbb; padding: 0.3em 0.6em; text-align: left; }
hr { border: 0; border-top: 1px solid #ccc; margin: 1.5em 0; }
.subtitle { color: #555; font-size: 1.05em; margin-top: -0.3em; }
.meta { color: #666; font-size: 0.9em; }
"""

H2_PATTERN = re.compile(
    r'<h2[^>]*\bid="(?P<id>[^"]+)"[^>]*>(?P<text>.*?)</h2>', re.DOTALL
)
TAG_PATTERN = re.compile(r"<[^>]+>")


@dataclass(frozen=True, slots=True)
class Document:
    """One markdown source destined to become one EPUB chapter."""

    path: Path
    title: str
    section: str | None
    file_name: str


@dataclass(frozen=True, slots=True)
class Chapter:
    """A rendered chapter plus the sub-headings used to build nested navigation."""

    document: Document
    item: epub.EpubHtml
    subheadings: tuple[tuple[str, str], ...]


def find_repo_root(start: Path) -> Path:
    """Return the nearest ancestor of ``start`` containing an ``openspec`` directory.

    Falls back to the git top level, then to ``start`` itself.
    """
    for candidate in (start, *start.parents):
        if (candidate / "openspec").is_dir():
            return candidate
    try:
        top = subprocess.run(
            ("git", "rev-parse", "--show-toplevel"),
            cwd=start,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    except (subprocess.CalledProcessError, OSError):
        return start
    return Path(top) if top else start


def list_changes(changes_dir: Path) -> list[str]:
    """Return the names of active (non-archived) changes, sorted."""
    if not changes_dir.is_dir():
        return []
    return sorted(
        entry.name
        for entry in changes_dir.iterdir()
        if entry.is_dir() and entry.name != "archive"
    )


def humanize(stem: str) -> str:
    """Turn a kebab/snake-case file stem into a title, respecting known acronyms.

    >>> humanize("proposal")
    'Proposal'
    >>> humanize("python-frontend")
    'Python Frontend'
    >>> humanize("ir-lowering")
    'IR Lowering'
    """
    words = stem.replace("-", " ").replace("_", " ").title().split()
    return " ".join(ACRONYMS.get(word, word) for word in words)


def artifact_sort_key(stem: str) -> tuple[int, str]:
    """Order known artifacts first, then everything else alphabetically.

    >>> artifact_sort_key("proposal")
    (0, 'proposal')
    >>> artifact_sort_key("zebra")[0]
    3
    """
    if stem in ARTIFACT_ORDER:
        return (ARTIFACT_ORDER.index(stem), stem)
    return (len(ARTIFACT_ORDER), stem)


def collect_documents(change_dir: Path) -> list[Document]:
    """Collect a change's markdown artifacts in reading order.

    Top-level artifacts come first (proposal, design, tasks, then any others),
    followed by one document per capability spec under ``specs/``.
    """
    documents: list[Document] = []

    top_level = sorted(
        (p for p in change_dir.glob("*.md") if p.is_file()),
        key=lambda p: artifact_sort_key(p.stem),
    )
    documents.extend(
        Document(
            path=path,
            title=humanize(path.stem),
            section=None,
            file_name=f"{path.stem}.xhtml",
        )
        for path in top_level
    )

    specs_dir = change_dir / "specs"
    for spec_path in sorted(specs_dir.glob("**/*.md")):
        capability = spec_path.parent.relative_to(specs_dir).as_posix()
        slug = capability.replace("/", "-")
        documents.append(
            Document(
                path=spec_path,
                title=humanize(slug),
                section="Specifications",
                file_name=f"spec-{slug}.xhtml",
            )
        )

    return documents


def collect_context_documents(repo_root: Path) -> list[Document]:
    """Collect repo-level supporting documents that provide project context."""
    documents: list[Document] = []
    for name in CONTEXT_DOCUMENTS:
        path = repo_root / name
        if path.is_file() and path.stat().st_size > 0:
            title = "Project Context" if path.stem == "CLAUDE" else humanize(path.stem)
            documents.append(
                Document(
                    path=path,
                    title=title,
                    section="Appendix",
                    file_name=f"appendix-{path.stem.lower()}.xhtml",
                )
            )
    return documents


def render_markdown(text: str) -> str:
    """Convert markdown to XHTML-compatible markup suitable for EPUB."""
    converter = markdown.Markdown(
        extensions=list(MARKDOWN_EXTENSIONS),
        output_format="xhtml",
    )
    return converter.convert(text)


def extract_subheadings(html: str) -> tuple[tuple[str, str], ...]:
    """Return ``(anchor_id, text)`` for each ``<h2>`` in rendered HTML.

    >>> extract_subheadings('<h2 id="why">Why</h2>')
    (('why', 'Why'),)
    """
    found: list[tuple[str, str]] = []
    for match in H2_PATTERN.finditer(html):
        text = TAG_PATTERN.sub("", match.group("text")).strip()
        if text:
            found.append((match.group("id"), text))
    return tuple(found)


def git_author(repo_root: Path) -> str:
    """Return the configured git user name, or a neutral default."""
    try:
        name = subprocess.run(
            ("git", "config", "user.name"),
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    except (subprocess.CalledProcessError, OSError):
        return "Unknown"
    return name or "Unknown"


def build_title_page(change_name: str, documents: list[Document], today: str) -> str:
    """Build the XHTML for the opening title page."""
    items = "\n".join(
        f"<li>{doc.title}"
        + (f' <span class="meta">({doc.section})</span>' if doc.section else "")
        + "</li>"
        for doc in documents
    )
    return (
        f"<h1>{change_name}</h1>\n"
        f'<p class="subtitle">OpenSpec change report</p>\n'
        f'<p class="meta">Generated {today}</p>\n'
        f"<hr />\n"
        f"<h2>Contents</h2>\n"
        f"<ul>\n{items}\n</ul>\n"
    )


def build_toc(chapters: list[Chapter]) -> list[Any]:
    """Build a nested table of contents, grouping chapters by their section.

    Chapters without a section become top-level entries; consecutive chapters
    sharing a section are nested beneath it. A chapter with ``<h2>`` headings
    gets those as a further level of navigation.
    """
    toc: list[Any] = []
    bucket: list[Any] = []
    current_section: str | None = None

    def entry_for(chapter: Chapter) -> Any:
        uid = Path(chapter.item.file_name).stem
        link = epub.Link(chapter.item.file_name, chapter.document.title, uid)
        if not chapter.subheadings:
            return link
        children = tuple(
            epub.Link(
                f"{chapter.item.file_name}#{anchor}",
                text,
                f"{uid}-{anchor}",
            )
            for anchor, text in chapter.subheadings
        )
        return (link, children)

    def flush() -> None:
        nonlocal bucket, current_section
        if bucket and current_section is not None:
            toc.append((epub.Section(current_section), tuple(bucket)))
        bucket = []

    for chapter in chapters:
        section = chapter.document.section
        if section is None:
            flush()
            current_section = None
            toc.append(entry_for(chapter))
            continue
        if section != current_section:
            flush()
            current_section = section
        bucket.append(entry_for(chapter))

    flush()
    return toc


def build_book(
    change_name: str,
    documents: list[Document],
    author: str,
    today: str,
) -> epub.EpubBook:
    """Assemble the EPUB from the collected documents."""
    book = epub.EpubBook()
    book.set_identifier(f"compylr-{change_name}-{uuid.uuid4()}")
    book.set_title(f"compylr - {change_name}")
    book.set_language("en")
    book.add_author(author)
    book.add_metadata("DC", "date", today)
    book.add_metadata(
        "DC", "description", f"Planning artifacts for change {change_name}"
    )

    style = epub.EpubItem(
        uid="style",
        file_name="style/main.css",
        media_type="text/css",
        content=STYLESHEET,
    )
    book.add_item(style)

    title_page = epub.EpubHtml(title="Title", file_name="title.xhtml", lang="en")
    title_page.content = build_title_page(change_name, documents, today)
    title_page.add_item(style)
    book.add_item(title_page)

    chapters: list[Chapter] = []
    for doc in documents:
        html = render_markdown(doc.path.read_text(encoding="utf-8"))
        item = epub.EpubHtml(title=doc.title, file_name=doc.file_name, lang="en")
        item.content = f"<h1>{doc.title}</h1>\n{html}"
        item.add_item(style)
        book.add_item(item)
        chapters.append(
            Chapter(document=doc, item=item, subheadings=extract_subheadings(html))
        )

    book.toc = build_toc(chapters)
    book.spine = [title_page, *(chapter.item for chapter in chapters)]
    book.add_item(epub.EpubNcx())
    book.add_item(epub.EpubNav())
    return book


def resolve_change_name(explicit: str | None, changes_dir: Path) -> str:
    """Determine which change to render, erroring clearly when it is ambiguous."""
    if explicit:
        return explicit
    available = list_changes(changes_dir)
    if not available:
        raise SystemExit(f"No active changes found in {changes_dir}")
    if len(available) > 1:
        listed = "\n  ".join(available)
        raise SystemExit(
            f"Multiple active changes; name the one to render:\n  {listed}"
        )
    return available[0]


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "change",
        nargs="?",
        help="Change name to render. Defaults to the sole active change.",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="Output path. Defaults to reports/<change>.epub in the repo root.",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="List active changes and exit.",
    )
    parser.add_argument(
        "--no-context",
        action="store_true",
        help="Omit repo-level supporting documents (CLAUDE.md, README.md).",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Entry point. Returns a process exit code."""
    args = parse_args(argv)
    repo_root = find_repo_root(Path.cwd())
    changes_dir = repo_root / "openspec" / "changes"

    if args.list:
        for name in list_changes(changes_dir):
            print(name)
        return 0

    change_name = resolve_change_name(args.change, changes_dir)
    change_dir = changes_dir / change_name
    if not change_dir.is_dir():
        raise SystemExit(f"Change directory not found: {change_dir}")

    documents = collect_documents(change_dir)
    if not documents:
        raise SystemExit(f"No markdown artifacts found in {change_dir}")
    if not args.no_context:
        documents.extend(collect_context_documents(repo_root))

    output = args.output or (repo_root / "reports" / f"{change_name}.epub")
    output.parent.mkdir(parents=True, exist_ok=True)

    today = dt.date.today().isoformat()
    book = build_book(change_name, documents, git_author(repo_root), today)
    epub.write_epub(str(output), book)

    size_kb = output.stat().st_size / 1024
    print(f"Wrote {output} ({size_kb:.1f} KiB)")
    print(f"{len(documents)} chapters:")
    for doc in documents:
        label = f"{doc.section}/{doc.title}" if doc.section else doc.title
        print(f"  - {label}  <- {doc.path.relative_to(repo_root)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
