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
# Permissions are skipped, and that is only defensible because of where this
# runs: a worktree of its own, on a branch of its own, with its own target
# directory. The model needs a shell — the prompt asks it to compile and test
# what it wrote, which is most of what makes the output worth having — and 792
# permission prompts is not a batch. What keeps it honest is downstream: every
# card is reverted unless the gate passes AND the only file it touched was its
# own, so a shell used for anything else leaves nothing behind.
#
# The narrow gate is deliberate. `cargo test --workspace` takes minutes and
# the client alone links half a gigabyte; per card that is the difference
# between a batch overnight and a batch over a week. The full gate runs once,
# on the branch, before anything is merged.
set -u
PKGS=${1:?usage: run.sh <packages-dir> [count] [model]}
COUNT=${2:-10}
MODEL=${3:-gemini-3.8-flash-medium}
ROOT=$(git rev-parse --show-toplevel)
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

  status=$(python3 "$HERE/verdict.py" "$verdict" status 2>/dev/null)
  if [ $rc -ne 0 ]; then
    echo "  agy failed (rc=$rc) — see $LOG/$slug.err"
    status=refused
  fi

  # The gate. Narrow, but it runs before anything is kept, and a card that
  # only compiles is not a card that passed.
  if [ "$status" = "implemented" ] || [ "$status" = "partial" ]; then
    if ( cd "$ROOT" \
          && cargo check -p baylee-cards --quiet \
          && cargo test -p baylee-cards --quiet \
          && cargo run -q -p xtask -- validate ) >> "$LOG/$slug.gate" 2>&1; then
      # And the card the model was asked about is the only file it touched.
      changed=$(git -C "$ROOT" status --porcelain | awk '{print $2}')
      if [ "$changed" != "$card" ]; then
        echo "  touched more than its own file: $changed — reverting"
        git -C "$ROOT" checkout -- . && git -C "$ROOT" clean -fd -q
        status=refused
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
      status=refused
    fi
  fi

  if [ "$status" = "refused" ]; then
    git -C "$ROOT" checkout -- . 2>/dev/null
    git -C "$ROOT" clean -fd -q 2>/dev/null
    python3 "$HERE/verdict.py" "$verdict" row "$slug" "$name" >> "$REFUSALS"
    echo "  refused — recorded"
  fi
done
echo "done: $done_n card(s) attempted"
