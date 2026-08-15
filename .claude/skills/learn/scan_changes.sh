#!/usr/bin/env bash
# scan_changes.sh — detection only, never edits.
#
# Surfaces the source files (and exact changed regions) that differ from the
# last commit, so /learn knows where to concentrate teaching comments. The
# most-recently-modified file is listed first: that's almost certainly the
# code you're actively editing right now.
#
# Sections emitted:
#   BASELINE            what we diff against (a commit, or the empty tree if
#                       the repo has no commits yet)
#   CHANGED FILES       newline list of source paths, freshest first
#   DIFF                unified hunks for tracked changes + full body of new
#                       (untracked) files
#
# Usage: scan_changes.sh

set -euo pipefail

# --- config ------------------------------------------------------------------
# Only real source carries teaching comments; skip vendored crates, build
# artifacts, IDE metadata, and lockfiles. Add extensions here as the project
# grows (e.g. '*.toml' once you hand-edit build config worth annotating).
SRC_PATHSPEC=('*.rs' '*.py' ':(exclude)vendored/**' ':(exclude)target/**')

# --- locate repo -------------------------------------------------------------
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# --- pick a baseline ---------------------------------------------------------
# A brand-new repo has no HEAD. Fall back to git's canonical empty-tree object
# so "everything is new" still diffs cleanly.
if git rev-parse --verify -q HEAD >/dev/null; then
  BASE="$(git rev-parse HEAD)"
  BASE_LABEL="HEAD ($(git rev-parse --short HEAD))"
else
  BASE="$(git hash-object -t tree /dev/null)" # the empty tree
  BASE_LABEL="empty tree (no commits yet — treating all source as new)"
fi

echo "=== BASELINE ==="
echo "$BASE_LABEL"
echo

# --- portable mtime ----------------------------------------------------------
# macOS (BSD stat) and Linux (GNU stat) disagree on flags; detect once.
mtime() {
  if stat -f '%m' "$1" >/dev/null 2>&1; then
    stat -f '%m' "$1" # BSD / macOS
  else
    stat -c '%Y' "$1" # GNU / Linux
  fi
}

# --- collect changed source files -------------------------------------------
# `git status --porcelain` reports staged, unstaged, AND untracked uniformly,
# which matters here because nothing may be committed yet. We re-run the source
# pathspec filter through `git diff`/`ls-files` so excludes apply consistently.
# (read loop instead of mapfile — macOS still ships bash 3.2)
CHANGED=()
while IFS= read -r line; do
  [ -n "$line" ] && CHANGED+=("$line")
done < <(
  {
    # tracked changes vs baseline (staged + unstaged)
    git diff --name-only "$BASE" -- "${SRC_PATHSPEC[@]}"
    # untracked files matching the same pathspec
    git ls-files --others --exclude-standard -- "${SRC_PATHSPEC[@]}"
  } | sort -u
)

if [ "${#CHANGED[@]}" -eq 0 ]; then
  echo "=== CHANGED FILES ==="
  echo "(none — working tree matches baseline for tracked source types)"
  exit 0
fi

# Sort by mtime, newest first → "what you're editing now" floats to the top.
echo "=== CHANGED FILES (most-recently-modified first) ==="
for f in "${CHANGED[@]}"; do
  [ -f "$f" ] || continue # skip deletions
  printf '%s\t%s\n' "$(mtime "$f")" "$f"
done | sort -rn | cut -f2-
echo

# --- show the actual changes -------------------------------------------------
echo "=== DIFF ==="
for f in "${CHANGED[@]}"; do
  if git ls-files --error-unmatch "$f" >/dev/null 2>&1 \
     && ! git ls-files --others --exclude-standard -- "$f" | grep -q .; then
    # tracked: show unified hunks against baseline
    git --no-pager diff "$BASE" -- "$f"
  elif [ -f "$f" ]; then
    # untracked/new: show full body as an against-/dev/null diff
    git --no-pager diff --no-index -- /dev/null "$f" || true
  fi
done
