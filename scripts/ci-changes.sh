#!/usr/bin/env bash
# What a change touches, for `ci.yml`'s `changes` job: reads changed paths on
# stdin, one per line, and prints `key=true|false` lines for $GITHUB_OUTPUT.
#
#   code  anything compiled, packaged or run by a Rust job: crates, xtask,
#         Cargo files, scripts, data, LICENSE/NOTICE (both are packaged), ...
#   docs  Markdown and docs/. Not inert: tests `include_str!` docs/card-dsl.md,
#         docs/llm-seat.md, docs/terms-placeholder*, and the gateway reads
#         docs/ at test time, so a docs change still runs the debug suite.
#   web   web/feedback/, the feedback UI, which only its own job builds.
#   ci    the workflows, their actions and this file: a CI change runs all of
#         CI, because that is the only way to see it work.
#
# Paths no job reads (agent scratch directories, the other web/ pages) set
# nothing. An empty list, or a diff that could not be computed, is the
# caller's to treat as "everything" (`ci.yml` does).
#
# Locally:  git diff --name-only origin/main | scripts/ci-changes.sh
set -euo pipefail

code=false docs=false web=false ci=false
while IFS= read -r path; do
    [ -n "$path" ] || continue
    case $path in
        .github/* | scripts/ci-changes.sh) ci=true ;;
        web/feedback/*) web=true ;;
        .claude/* | .agents/* | .codex/* | .junie/* | web/*) ;;
        docs/* | *.md) docs=true ;;
        *) code=true ;;
    esac
done
printf 'code=%s\ndocs=%s\nweb=%s\nci=%s\n' "$code" "$docs" "$web" "$ci"
