#!/bin/zsh
# Drives Antigravity (`agy`) through a queue of card task packages.
#
# One card per invocation of the model, one commit per card that passes, and
# nothing at all committed for a card that does not: a bad card has to be
# removable on its own, and a batch that landed as one commit would make that
# a revert of the whole night's work.
#
# Usage:  tools/cardbatch/run.sh <packages-dir> [count] [model]
#
# The default model is gemini-3.8-flash-high. The suffix is reasoning effort,
# not a different model: a card is a small amount of code that has to be right
# in a way a compiler cannot check, which is the shape of task that repays
# thinking rather than throughput.
#
# Permissions are skipped, and that is only defensible because of where this
# runs: a *clone* of its own, on a branch of its own, with its own target
# directory. The model needs a shell — the prompt asks it to compile what it
# wrote, which is most of what makes the output worth having — and 792
# permission prompts is not a batch. What keeps it honest is downstream: every
# card is reverted unless the gate passes AND the only file it touched was its
# own, so a shell used for anything else leaves nothing behind.
#
# A clone and not a git worktree, which is what this was first, and the
# difference is the whole safety argument. A worktree's `.git` is a *file*
# pointing back at the real repository, so `git rev-parse --git-common-dir`
# answers with the main checkout's `.git` — and `agy` resolves the project it
# is working on that way. The first trial run therefore edited
# /Users/viktor/Projects/baylee while believing it was sandboxed: the guard
# below saw a clean tree (a worktree's `git status` cannot see another
# worktree's files), reverted nothing, and would have kept doing that for 792
# cards. A clone's `.git` is a directory, so the common dir is its own, and
# the guard is a guard. `refuse_shared_checkout` below makes the failure loud
# rather than silent if this is ever pointed at a worktree again.
#
# The narrow gate is deliberate. `cargo test --workspace` takes minutes and
# the client alone links half a gigabyte; per card that is the difference
# between a batch overnight and a batch over a week. The full gate runs once,
# on the branch, before anything is merged.
set -u
PKGS=${1:?usage: run.sh <packages-dir> [count] [model]}
COUNT=${2:-10}
MODEL=${3:-gemini-3.8-flash-high}
ROOT=$(git rev-parse --show-toplevel)

# The isolation, asserted rather than assumed. `agy` finds the project it edits
# through the git *common* directory, so in a worktree it walks out of here and
# into the main checkout — see the note at the top. If this is not a clone,
# stop: everything below (the per-card revert, the "touched only its own file"
# check, skipped permissions) is arguing about a tree the model is not editing.
if [ ! -d "$ROOT/.git" ]; then
  print -u2 "refuse_shared_checkout: $ROOT/.git is not a directory, so this is a"
  print -u2 "  worktree or a submodule, and agy would edit whatever holds"
  print -u2 "  $(git rev-parse --git-common-dir) instead."
  print -u2 "  instead. Clone the repository and run there."
  exit 2
fi

HERE=$ROOT/tools/cardbatch
LOG=$ROOT/target/cardbatch
mkdir -p "$LOG"
REFUSALS=$ROOT/data/card-refusals.tsv
if [ ! -f "$REFUSALS" ]; then
  printf 'slug\tname\tstatus\toracle_sentence\tcannot_say\tnearest_existing\n' > "$REFUSALS"
fi

# A dirty tree would make "did the model change anything" unanswerable.
if [ -n "$(git -C "$ROOT" status --porcelain)" ]; then
  echo "working tree is not clean — refusing to start" >&2
  exit 1
fi

done_n=0
for dir in "$PKGS"/*(/); do
  [ $done_n -ge $COUNT ] && break
  slug=${dir:t}
  card=crates/baylee-cards/src/cards/$slug.rs
  # Already finished by an earlier run, or by codegen's own readers.
  grep -q '// GENERATED STUB' "$ROOT/$card" 2>/dev/null || continue
  done_n=$((done_n+1))
  name=$(sed -n '1s/^\/\/! \([^—]*\).*/\1/p' "$ROOT/$card" | sed 's/ *$//')
  echo "=== [$done_n/$COUNT] $slug — $name"

  verdict=$LOG/$slug.json
  ( cd "$ROOT" && agy -p "$(cat "$dir/PROMPT.md")" \
      --model "$MODEL" \
      --mode accept-edits \
      --dangerously-skip-permissions \
      --output-format json \
      --json-schema "$HERE/verdict.schema.json" \
      --print-timeout 20m \
      > "$verdict" 2> "$LOG/$slug.err" )
  rc=$?

  verdict_status=$(python3 "$HERE/verdict.py" "$verdict" status 2>/dev/null)
  if [ $rc -ne 0 ]; then
    echo "  agy failed (rc=$rc) — see $LOG/$slug.err"
    verdict_status=refused
  fi

  # The gate. Narrow, but it runs before anything is kept, and a card that
  # only compiles is not a card that passed.
  if [ "$verdict_status" = "implemented" ] || [ "$verdict_status" = "partial" ]; then
    if ( cd "$ROOT" \
          && cargo check -p baylee-cards --quiet \
          && cargo test -p baylee-cards --quiet \
          && cargo run -q -p xtask -- validate ) >> "$LOG/$slug.gate" 2>&1; then
      # And the card the model was asked about is the only file it touched.
      changed=$(git -C "$ROOT" status --porcelain | awk '{print $2}')
      if [ "$changed" != "$card" ]; then
        echo "  touched more than its own file: $changed — reverting"
        git -C "$ROOT" checkout -- . && git -C "$ROOT" clean -fd -q
        verdict_status=refused
      else
        git -C "$ROOT" add "$card"
        git -C "$ROOT" commit -q -m "feat(cards): $name

Implemented by $MODEL through tools/cardbatch. Unreviewed: the narrow gate
says it compiles and the data tests pass, which is not the same as the card
being right.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
        echo "  committed"
      fi
    else
      echo "  gate failed — reverting, see $LOG/$slug.gate"
      git -C "$ROOT" checkout -- . && git -C "$ROOT" clean -fd -q
      verdict_status=refused
    fi
  fi

  if [ "$verdict_status" = "refused" ]; then
    git -C "$ROOT" checkout -- . 2>/dev/null
    git -C "$ROOT" clean -fd -q 2>/dev/null
    python3 "$HERE/verdict.py" "$verdict" row "$slug" "$name" >> "$REFUSALS"
    echo "  refused — recorded"
  fi
done
echo "done: $done_n card(s) attempted"
