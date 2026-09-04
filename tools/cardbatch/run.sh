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
# directory. The model edits files unattended — 459 permission prompts is not a
# batch — and it is told to run no shell commands at all, so the only thing it
# may do here is write one card. What keeps it honest is downstream: every
# card is reverted unless the gate passes AND the only file it touched was its
# own, so a shell used for anything else leaves nothing behind.
#
# A clone and not a git worktree, and neither of those is the isolation. This
# was a worktree first, and the first trial run edited
# /Users/viktor/Projects/baylee while every check here inspected the worktree
# and found it clean. Replacing it with a clone did not fix that: the second
# trial run escaped again, from a checkout whose `.git` is its own directory.
# So the mechanism is not the git common dir — `agy` does not resolve the
# project from the working directory at all. It carries its own project
# registry (`--project` / `--new-project`), and given neither it reuses the
# most recent one, which on this machine is the real checkout. `--new-project
# --add-dir "$ROOT"` below is what binds a session to this tree;
# `upstream_state` is what proves it, per card, rather than trusting it.
# `refuse_shared_checkout` stays because the per-card revert still needs a
# `git status` that can see every file the model may have written.
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
# Where the work goes when it is done. This clone lives in a scratchpad under
# /private/tmp, which macOS cleans and a reboot loses: a night of card commits
# sitting only in *this* `.git` is a night with no copy of itself. `origin` is
# the real checkout, and the branch is not the one checked out over there, so
# a plain push is accepted.
BRANCH=$(git -C "$ROOT" rev-parse --abbrev-ref HEAD)
push_home() {
  git -C "$ROOT" push -q origin "$BRANCH" 2>/dev/null \
    || echo "  push to origin failed — the commit is still here, on $BRANCH"
}
# The ledger is written into `target/`, not into `data/`, and only lands in the
# repository when the run ends. `data/card-refusals.tsv` is a *tracked* file in
# the working tree, so the `git checkout -- .` that reverts a failed card
# reverts the ledger with it: every refusal was faithfully recorded and then
# erased by the next card's revert, leaving one row out of however many. Under
# `target/` nothing in this script can reach it.
LEDGER=$ROOT/data/card-refusals.tsv
REFUSALS=$LOG/refusals.tsv
: > "$REFUSALS"
if [ ! -f "$LEDGER" ]; then
  printf 'slug\tname\tstatus\toracle_sentence\tcannot_say\tnearest_existing\n' > "$LEDGER"
fi

# A dirty tree would make "did the model change anything" unanswerable.
if [ -n "$(git -C "$ROOT" status --porcelain)" ]; then
  echo "working tree is not clean — refusing to start" >&2
  exit 1
fi

# The second isolation guard, and the one that matters: `agy` does not take the
# working directory as the thing it edits. It carries its own notion of a
# *project* (`--project`, `--new-project`), and given none it reuses the most
# recent one — which on this machine is the real checkout. So the first batch
# wrote its cards into /Users/viktor/Projects/baylee while every check in this
# script inspected the clone, found it clean, and recorded `no-edit`; the card
# the model had actually written was sitting in somebody else's working tree.
# `--new-project` below binds the session to this directory. This check is what
# turns that from a hope into a fact: the upstream checkout is photographed
# before each card and compared after, and any difference stops the run, because
# nothing here can safely revert a tree it does not own.
#
# Photographed *narrowly*, though, and that is not laziness. This batch runs for
# hours while somebody works in the upstream checkout, so a whole-tree `git
# status` plus `rev-parse HEAD` makes every ordinary commit and every edited
# file over there look like an escape — the first probe stopped on exactly that,
# a commit of mine to `xtask/`. What the model can write is a card, so the watch
# is the three card directories: an escape lands there, and the upstream's own
# work does not. `rev-parse` is gone entirely; the model has no shell and cannot
# commit.
UPSTREAM=$(git -C "$ROOT" remote get-url origin 2>/dev/null)
WATCHED=(crates/baylee-cards crates/baylee-cards-codegen crates/baylee-cards-dsl)
upstream_state() {
  if [ -n "$UPSTREAM" ] && [ -d "$UPSTREAM/.git" ]; then
    git -C "$UPSTREAM" status --porcelain -- $WATCHED
  fi
}

done_n=0
# A run that keeps reporting cards it never wrote is a broken prompt, not a
# difficult pool, and it costs about six minutes of model time per card to keep
# finding that out. Three in a row and the batch stops rather than spending the
# other four hundred slots on the same mistake.
NO_EDIT_STREAK=0
NO_EDIT_LIMIT=3
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
  upstream_before=$(upstream_state)
  ( cd "$ROOT" && agy -p "$(cat "$dir/PROMPT.md")" \
      --model "$MODEL" \
      --new-project \
      --add-dir "$ROOT" \
      --mode accept-edits \
      --dangerously-skip-permissions \
      --output-format json \
      --json-schema "$HERE/verdict.schema.json" \
      --print-timeout 20m \
      > "$verdict" 2> "$LOG/$slug.err" )
  rc=$?

  if [ "$(upstream_state)" != "$upstream_before" ]; then
    print -u2 "escaped_the_clone: the model wrote cards into $UPSTREAM, not this clone."
    print -u2 "  Nothing here may revert a tree it does not own. Go and look at it:"
    print -u2 "    git -C $UPSTREAM status -- $WATCHED"
    exit 3
  fi

  verdict_status=$(python3 "$HERE/verdict.py" "$verdict" status 2>/dev/null)
  # What *this* script concluded, which is not always what the model claimed.
  # The ledger records this one: a card the model called implemented and never
  # wrote is a different thing from a card it declined, and reading the first
  # as the second is how a batch quietly loses work.
  outcome=$verdict_status
  if [ $rc -ne 0 ]; then
    echo "  agy failed (rc=$rc) — see $LOG/$slug.err"
    verdict_status=refused
    outcome=agy-failed
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
      if [ -z "$changed" ]; then
        # Nothing to revert and nothing to keep: the model reported a card it
        # never wrote. The first batch hit this after its own `cargo check`
        # deadlocked, and the message read "touched more than its own file:"
        # with an empty list, which is the opposite of what happened.
        echo "  reported a card it never wrote — recording"
        verdict_status=refused
        outcome=no-edit
        NO_EDIT_STREAK=$((NO_EDIT_STREAK+1))
      elif [ "$changed" != "$card" ]; then
        echo "  touched more than its own file: $changed — reverting"
        git -C "$ROOT" checkout -- . && git -C "$ROOT" clean -fd -q
        verdict_status=refused
        outcome=stray-edits
      else
        git -C "$ROOT" add "$card"
        git -C "$ROOT" commit -q -m "feat(cards): $name

Implemented by $MODEL through tools/cardbatch. Unreviewed: the narrow gate
says it compiles and the data tests pass, which is not the same as the card
being right.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
        echo "  committed"
        NO_EDIT_STREAK=0
        push_home
      fi
    else
      echo "  gate failed — reverting, see $LOG/$slug.gate"
      git -C "$ROOT" checkout -- . && git -C "$ROOT" clean -fd -q
      verdict_status=refused
      outcome=gate-failed
    fi
  fi

  if [ "$verdict_status" = "refused" ]; then
    git -C "$ROOT" checkout -- . 2>/dev/null
    git -C "$ROOT" clean -fd -q 2>/dev/null
    python3 "$HERE/verdict.py" "$verdict" row "$slug" "$name" "$outcome" >> "$REFUSALS"
    echo "  $outcome — recorded"
  fi

  if [ $NO_EDIT_STREAK -ge $NO_EDIT_LIMIT ]; then
    echo "$NO_EDIT_STREAK cards in a row reported without an edit — stopping."
    echo "That is the prompt, not the pool. Fix it before spending the rest."
    break
  fi
done

# Now that no revert can follow, the run's refusals join the tracked ledger as
# one commit. Its two load-bearing columns are `cannot_say` and
# `nearest_existing`: together they are a work item in the DSL's own words,
# which is the whole reason a refusal is worth as much as a card.
if [ -s "$REFUSALS" ]; then
  cat "$REFUSALS" >> "$LEDGER"
  git -C "$ROOT" add "${LEDGER#$ROOT/}"
  git -C "$ROOT" commit -q -m "chore(cards): $(wc -l < "$REFUSALS" | tr -d ' ') refusal(s) from a $MODEL batch

Every one of these was reverted, so the cards are still generated stubs. The
rows say why: \`cannot_say\` names what the DSL cannot express and
\`nearest_existing\` the closest variant that does exist.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
  push_home
  echo "ledger: $(wc -l < "$REFUSALS" | tr -d ' ') row(s) committed"
fi
echo "done: $done_n card(s) attempted"
