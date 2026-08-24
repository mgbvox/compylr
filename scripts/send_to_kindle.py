#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Email a document to a Kindle via SMTP.

Reads configuration from a ``.env`` file (never printed, never committed) and
sends the given file as an attachment to ``KINDLE_EMAIL``.

The attachment travels straight from disk to the SMTP socket, so nothing binary
passes through an LLM context window -- that keeps large documents cheap and
removes any chance of a transcription error corrupting the file.

Required in .env::

    KINDLE_EMAIL=you_abc123@kindle.com
    SMTP_USER=you@gmail.com
    SMTP_PASS=your-16-char-app-password

Optional (sensible Gmail defaults)::

    SMTP_HOST=smtp.gmail.com
    SMTP_PORT=587
    SMTP_FROM=you@gmail.com          # defaults to SMTP_USER

Usage::

    ./scripts/send_to_kindle.py reports/add-python-ir-lowering.epub
    ./scripts/send_to_kindle.py reports/foo.epub --dry-run
    ./scripts/send_to_kindle.py reports/foo.epub --subject "Spec review"
"""

from __future__ import annotations

import argparse
import mimetypes
import smtplib
import ssl
import sys
from email.message import EmailMessage
from pathlib import Path

# Amazon rejects Personal Document attachments above this size.
MAX_ATTACHMENT_BYTES = 25 * 1024 * 1024

DEFAULT_SMTP_HOST = "smtp.gmail.com"
DEFAULT_SMTP_PORT = 587


def parse_env_file(path: Path) -> dict[str, str]:
    """Parse a minimal ``.env`` file into a dict.

    Supports ``KEY=value`` lines, ``#`` comments, blank lines, and optional
    surrounding quotes. Deliberately tiny so the script stays dependency-free.

    >>> import tempfile, pathlib
    >>> with tempfile.NamedTemporaryFile("w", suffix=".env", delete=False) as fh:
    ...     _ = fh.write('# c\\nA=1\\nB="two"\\n\\n')
    >>> parse_env_file(pathlib.Path(fh.name)) == {"A": "1", "B": "two"}
    True
    """
    values: dict[str, str] = {}
    if not path.is_file():
        return values
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
            value = value[1:-1]
        values[key.strip()] = value
    return values


def find_repo_root(start: Path) -> Path:
    """Return the nearest ancestor containing a ``.env`` or ``openspec`` directory."""
    for candidate in (start, *start.parents):
        if (candidate / ".env").is_file() or (candidate / "openspec").is_dir():
            return candidate
    return start


def redact(address: str) -> str:
    """Mask an email address for safe logging.

    >>> redact("someone@kindle.com")
    's***@kindle.com'
    """
    local, at, domain = address.partition("@")
    if not at or not local:
        return "<redacted>"
    return f"{local[0]}***@{domain}"


def build_message(
    document: Path,
    sender: str,
    recipient: str,
    subject: str,
    body: str,
) -> EmailMessage:
    """Build the outgoing message with the document attached."""
    message = EmailMessage()
    message["From"] = sender
    message["To"] = recipient
    message["Subject"] = subject
    message.set_content(body)

    guessed, _ = mimetypes.guess_type(document.name)
    if document.suffix.lower() == ".epub":
        maintype, subtype = "application", "epub+zip"
    elif guessed:
        maintype, subtype = guessed.split("/", 1)
    else:
        maintype, subtype = "application", "octet-stream"

    message.add_attachment(
        document.read_bytes(),
        maintype=maintype,
        subtype=subtype,
        filename=document.name,
    )
    return message


def send(message: EmailMessage, config: dict[str, str]) -> None:
    """Deliver the message over STARTTLS."""
    host = config.get("SMTP_HOST", DEFAULT_SMTP_HOST)
    port = int(config.get("SMTP_PORT", DEFAULT_SMTP_PORT))
    context = ssl.create_default_context()
    with smtplib.SMTP(host, port, timeout=60) as server:
        server.starttls(context=context)
        server.login(config["SMTP_USER"], config["SMTP_PASS"])
        server.send_message(message)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("document", type=Path, help="File to send (e.g. an .epub).")
    parser.add_argument("--subject", help="Email subject. Defaults to the file stem.")
    parser.add_argument("--to", help="Override the KINDLE_EMAIL recipient.")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Validate config and report what would be sent, without connecting.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Entry point. Returns a process exit code."""
    args = parse_args(argv)

    document: Path = args.document
    if not document.is_file():
        raise SystemExit(f"No such file: {document}")
    size = document.stat().st_size
    if size > MAX_ATTACHMENT_BYTES:
        raise SystemExit(f"{document.name} is {size / 1024 / 1024:.1f} MB, over the 25 MB limit.")

    repo_root = find_repo_root(Path.cwd())
    config = parse_env_file(repo_root / ".env")
    recipient = args.to or config.get("KINDLE_EMAIL", "")

    absent = [key for key in ("SMTP_USER", "SMTP_PASS") if not config.get(key)]
    if not recipient:
        absent.insert(0, "KINDLE_EMAIL")
    if absent:
        raise SystemExit(
            "Missing required values in .env: "
            + ", ".join(absent)
            + "\nSee the module docstring for the expected keys."
        )

    sender = config.get("SMTP_FROM") or config["SMTP_USER"]
    subject = args.subject or document.stem
    body = (
        f"{document.name} ({size / 1024:.1f} KiB) sent from compylr.\n"
        "Delivered as a Kindle Personal Document."
    )

    print(f"document : {document} ({size / 1024:.1f} KiB)")
    print(f"from     : {redact(sender)}")
    print(f"to       : {redact(recipient)}")
    print(f"subject  : {subject}")

    if args.dry_run:
        print("dry-run  : configuration is complete; nothing was sent.")
        return 0

    message = build_message(document, sender, recipient, subject, body)
    send(message, config)
    print("status   : accepted by SMTP server")
    print(
        "note     : the sending address must be on your Kindle Approved Personal\n"
        "           Document E-mail List, or Amazon silently drops the message."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
