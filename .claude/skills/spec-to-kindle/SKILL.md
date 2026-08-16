---
name: spec-to-kindle
description: Runs the compylr spec review cycle - make sure the OpenSpec change artifacts are current, render them to a single EPUB, and hand it back as an attachment. Use this whenever the user wants to read, review, proofread, or take a change's spec away from the screen - phrases like "epub the change", "I want to read the proposal", "ship me the design doc", "render the spec", "put this on my e-reader", or "send the spec to my kindle". Also use it when they ask to regenerate a spec they already received, since the artifacts usually changed underneath. Reach for this even if they only say "kindle" or "epub" without naming a change.
---

# Spec to Kindle

The point of this cycle is to get planning artifacts off the terminal and into a form that reads
well away from a screen. Specs are written to be reviewed carefully; scrollback is a bad medium
for that. Three steps: confirm the artifacts are current, render, deliver.

## Delivery: attach it, do not email it

**The default is to attach the EPUB to your reply.** The user asked on 2026-08-16 for exactly
this — "no more direct emailing kindle" — so surfacing the file is the whole of step 3 unless
they say otherwise in the moment.

The mail path still exists and still works; it is described at the end for the case where someone
explicitly asks for it. Do not reach for it on your own initiative, and do not treat the skill's
name as authorisation: the name records where this cycle started, not how it delivers now.

## 1. Confirm the artifacts are current

Find the change first. If the user named one, use it. Otherwise:

```bash
openspec status --change "<name>" --json
```

or list what exists:

```bash
./scripts/render_change_epub.py --list
```

If exactly one active change exists, that is almost certainly the one they mean — use it
without asking. If several exist and the user was vague, ask which one rather than guessing;
sending the wrong spec wastes a round trip through their inbox.

Check whether the planning artifacts are complete. If `openspec status` shows missing
artifacts, the honest move is to say so and offer to fill the gaps via the propose or update
workflow — do not render a half-finished spec and let the user discover the holes on their
Kindle. If artifacts exist but the user has been editing code or specs since, just re-render;
it is cheap and the script always reads from disk.

## 2. Render the EPUB

```bash
./scripts/render_change_epub.py <change-name>
```

The script discovers artifacts itself (proposal, design, tasks, every capability spec under
`specs/`, plus `CLAUDE.md` as an appendix), builds a nested table of contents, and writes to
`reports/<change-name>.epub`. Useful flags: `-o` for a different output path, `--no-context`
to drop the repo-level appendix, `--list` to enumerate changes.

It runs via `uv` with PEP 723 inline dependencies, so no virtualenv setup is needed — the
shebang handles it. If `uv` is missing, `uv run scripts/render_change_epub.py` works too.

Read the summary it prints. It lists every chapter and its source file, which is the fastest
way to catch a spec that silently did not get included because it was in an unexpected place.

## 3. Hand it back

Attach `reports/<change-name>.epub` to your reply. That is the delivery step; there is nothing
outward-facing about it, so no confirmation is needed.

`reports/` is tracked in this repository, so commit the EPUB alongside the artifacts it was
rendered from. When a change lives on its own branch, commit it there, so the branch carries its
own rendered spec.

Say what the reader is getting — the change name, the chapter count, and anything that would
affect the reading order if several are sent at once. The chapter list the render script prints
is the fastest source for that.

## Sending by email — only when explicitly asked

Superseded as the default (see above). Kept because the machinery works and someone may want it.

```bash
./scripts/send_to_kindle.py reports/<change-name>.epub
```

Sending is outward-facing and lands on someone's device, so confirm before the first send in a
session even when asked. Use `--dry-run` to validate configuration and print the redacted
sender/recipient without connecting to anything — a good way to check setup before committing
to a real send.

**Use the script, not the Gmail MCP.** The MCP requires the attachment inlined as base64 in the
tool call, which for even a small EPUB is tens of thousands of characters — that overflows the
response output limit and risks a transcription error silently corrupting the zip. The script
streams the file from disk to the SMTP socket, so the bytes never pass through a context window
and arrive byte-identical. This matters more as specs grow.

Configuration lives in `.env`: `KINDLE_EMAIL`, `SMTP_USER`, `SMTP_PASS` (a Gmail **app
password**, not the account password — Google rejects plain passwords for SMTP), and optionally
`SMTP_HOST`, `SMTP_PORT`, `SMTP_FROM`. If required keys are missing, the script says exactly
which ones and exits non-zero rather than half-sending.

`.env` is gitignored and must stay that way. Do not print the whole file, do not paste its
contents into a message, and do not `git add` it — if you ever find it staged, unstage it with
`git rm --cached .env`, because `.gitignore` does not retroactively untrack files already in
the index. The script only ever prints redacted addresses, so its output is safe to show.

## When delivery does not show up

Nearly always one of two things, and worth mentioning to the user rather than retrying blindly:

- The sending Gmail address is not on the Kindle **Approved Personal Document E-mail List**.
  Amazon silently drops mail from unapproved senders. They fix this in Amazon account settings
  under Preferences → Personal Document Settings.
- The address in `.env` is the `@kindle.com` address but a typo'd variant. Confirm the value
  reads as expected without echoing it in full.

## Verifying before you claim success

A clean exit means the SMTP server accepted the message — not that Amazon delivered it to the
device. Say "sent" rather than "it's on your Kindle", and mention it can take a few minutes to
appear. Overstating delivery sends the user hunting for a document that is still in flight.
