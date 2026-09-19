#!/bin/bash
# fmt and clippy over what changed, and nothing else.
#
# `gate-rules.sh` is a filter over *crates* — it excludes the client and runs
# everything else. This is the filter one level finer: it asks git what the
# working tree has actually changed and formats exactly those files, then
# lints exactly the crates that own them.
#
# The two halves are not equally fine-grained, and that is rustfmt and clippy
# rather than a choice made here. `rustfmt` takes a file. **Clippy cannot**:
# it lints by compiling, so its smallest unit is the crate, and
# `cargo clippy -p <crate> --all-targets` is the narrowest thing that exists.
# Asking for less would be asking for a check that does not run.
#
# This is for *while working*. It is not a gate: it says nothing about a crate
# that merely depends on what changed, and a new enum variant breaks those and
# not this. Run the full clippy and the full test before a push — measured on
# 19.09.2026, a narrow run in the client worktree had never touched three
# integration targets or a whole crate that links the one being edited.
#
# Usage:  scripts/gate-touched.sh            # uncommitted changes
#         scripts/gate-touched.sh <rev>      # everything since <rev>
set -u
cd "$(dirname "$0")/.."

if [ $# -ge 1 ]; then
    files=$(git diff --name-only --diff-filter=d "$1" -- '*.rs')
else
    # Tracked changes plus untracked files: a brand-new card file is exactly
    # the thing a "what did I touch" question must not miss.
    files=$(
        { git diff --name-only --diff-filter=d -- '*.rs'
          git diff --name-only --diff-filter=d --cached -- '*.rs'
          git ls-files --others --exclude-standard -- '*.rs'
        } | sort -u
    )
fi

if [ -z "$files" ]; then
    echo "gate-touched: no .rs file has changed — nothing to format or lint"
    exit 0
fi

n=$(printf '%s\n' "$files" | grep -c .)
echo "gate-touched: $n changed .rs file(s)"

# --- fmt, file by file -------------------------------------------------------
# `cargo fmt` has no way to name files, so rustfmt is called directly and the
# edition is passed explicitly: without it rustfmt assumes 2015 and reformats
# code this workspace does not write.
#
# One honest caveat on "exactly those files": rustfmt follows the `mod` tree
# below a file it is given, so naming a `lib.rs` or a `mod.rs` formats its
# whole subtree. That is harmless on a tree that is already formatted, and it
# is why this step is cheap either way — but it is more than was asked for,
# not less.
edition=$(grep -m1 '^edition' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')
printf '%s\n' "$files" | xargs rustfmt --edition "${edition:-2024}"
fmt_rc=$?
echo "STEP fmt rc=$fmt_rc"

# --- clippy, crate by crate --------------------------------------------------
# A file's crate is the nearest ancestor holding a Cargo.toml; its package name
# is what -p wants. Walking up beats a path prefix table, which would go stale
# the first time a crate is added.
crates=$(
    printf '%s\n' "$files" | while read -r f; do
        d=$(dirname "$f")
        while [ "$d" != "." ] && [ "$d" != "/" ]; do
            if [ -f "$d/Cargo.toml" ]; then
                grep -m1 '^name' "$d/Cargo.toml" | sed -E 's/.*"(.*)".*/\1/'
                break
            fi
            d=$(dirname "$d")
        done
    done | sort -u
)

if [ -z "$crates" ]; then
    echo "gate-touched: no crate owns those files — nothing to lint"
    exit $fmt_rc
fi

rc=$fmt_rc
for c in $crates; do
    echo "--- clippy -p $c"
    cargo clippy -p "$c" --all-targets -- -D warnings || rc=1
done
echo "STEP clippy rc=$rc"
echo "done rc=$rc  (crates: $(echo $crates | tr '\n' ' '))"
exit $rc
