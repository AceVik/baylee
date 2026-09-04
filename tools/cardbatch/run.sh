#!/bin/zsh
# Drives Antigravity (`agy`) through a queue of card task packages.
#
# One card per invocation of the model, one commit per card that passes, and
# nothing at all committed for a card that does not: a bad card has to be
# removable on its own, and a batch that landed as one commit would make that
# a revert of the whole night's work.
#
# Usage:  tools/cardbatch/run.sh <packages-dir> [count] [model,model,...]
#
# The models default to gemini-3.8-flash-high,claude-sonnet-4-6,
# claude-opus-4-6-thinking and are tried in that order, each until its own
# daily quota is spent. A `-high` or `-thinking` suffix is reasoning effort,
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
# on the branch, before anything is merged. Narrow was still not cheap enough
# to run between every pair of cards — see `GATE_EVERY` — so it now gates a
# chunk at a time and falls back to one card at a time only when a chunk
# fails.
set -u
PKGS=${1:?usage: run.sh <packages-dir> [count] [model,model,...]}
COUNT=${2:-10}
# The models to ask, in order of preference, comma-separated.
#
# A list rather than one name, because the wall this batch actually hits is a
# per-model daily quota: the first run met it after roughly fifteen cards and
# was told to come back in 2h19m. Those quotas are counted per model, so the
# answer to one being spent is the next one, and only a list with all of them
# spent is worth sleeping on.
typeset -a MODELS QUOTA_UNTIL
MODELS=(${(s:,:)${3:-gemini-3.8-flash-high,claude-sonnet-4-6,claude-opus-4-6-thinking}})
QUOTA_UNTIL=()
repeat ${#MODELS} QUOTA_UNTIL+=0
mi=1
MODEL=$MODELS[1]
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
# The ledger is written into `target/`, not into `data/`, and reaches the
# repository only as a commit — see `bank_refusals`. `data/card-refusals.tsv`
# is a *tracked* file in the working tree, so the revert that follows a failed
# card reverted the ledger with it: every refusal was faithfully recorded and
# then erased by the next card, leaving one row out of however many. Under
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
# is where a card has to land: `crates/baylee-cards/src/cards`. An escape puts a
# card there by construction, and the upstream's own work — client, codegen,
# tooling — does not. `rev-parse` is gone entirely; the model has no shell and
# cannot commit.
#
# The first narrowing still watched `baylee-cards-codegen` as well, and that was
# one directory too many: a `cargo fmt --all` upstream reformatted `landgen.rs`,
# and the batch stopped on card 7 believing the model had escaped. A guard that
# cries wolf at ordinary work next door gets switched off, which is worse than a
# guard that watches only the one place a card can appear.
UPSTREAM=$(git -C "$ROOT" remote get-url origin 2>/dev/null)
WATCHED=(crates/baylee-cards/src/cards)
upstream_state() {
  if [ -n "$UPSTREAM" ] && [ -d "$UPSTREAM/.git" ]; then
    git -C "$UPSTREAM" status --porcelain -- $WATCHED
  fi
}

done_n=0
# A run that keeps reporting cards it never wrote is a broken prompt, not a
# difficult pool, and it costs a card's worth of model time to keep finding
# that out. Three in a row and the batch stops rather than spending the other
# four hundred slots on the same mistake.
NO_EDIT_STREAK=0
NO_EDIT_LIMIT=3
# And the same for a model that will not answer at all. The quota is waited
# out above rather than counted here, so a run of failures this reaches is
# something else — an expired login, a model name that no longer exists — and
# none of those get better by trying the next card.
FAIL_STREAK=0
FAIL_LIMIT=5

# How many accepted cards may wait for one gate run.
#
# Measured on the first live batch: a *refused* card costs about 85 seconds,
# all of it model time, and an accepted one 2:39 to 3:46 — so `cargo check`
# plus `cargo test -p baylee-cards` plus `xtask validate` was costing roughly
# as much as writing the card did. Over 459 cards that is most of a working day
# spent recompiling one crate a card at a time.
#
# Cards are independent files, so the gate does not have to run between them.
# What it does have to preserve is what the per-card gate bought: one commit
# per card, and a bad card revertible on its own. `flush_gate` keeps both.
GATE_EVERY=${GATE_EVERY:-25}
# Accepted cards, and a copy of each. The copies are what make the failure path
# cheap: reverting a card is a `git checkout`, but *restoring* one is only
# possible if its text was kept somewhere first.
HOLD=$LOG/pending
rm -rf "$HOLD"
mkdir -p "$HOLD"
typeset -a PENDING PENDING_NAMES PENDING_MODELS

# How long to wait for the quota named in a verdict, in seconds.
#
# The message is `Individual quota reached. … Resets in 2h19m16s.` — a duration
# and not a wall-clock time, so it is read as one. A minute of slack on top,
# because a reset announced to the second is still a reset that has to have
# happened, and a quarter of an hour if the message ever stops carrying one.
quota_seconds() {
  local text h=0 m=0 s=0
  text=$(grep -o 'Resets in [0-9hms]*' "$1" | head -1)
  if [ -z "$text" ]; then
    print 900
    return
  fi
  [[ $text =~ '([0-9]+)h' ]] && h=$match[1]
  [[ $text =~ '([0-9]+)m' ]] && m=$match[1]
  [[ $text =~ '([0-9]+)s' ]] && s=$match[1]
  print $(( h * 3600 + m * 60 + s + 60 ))
}

# Takes the first model whose quota has reset, and says whether there was one.
# The order is the list's, not round-robin: a batch should come back to the
# cheap model as soon as it may, rather than drifting onto the expensive one
# and staying there.
pick_model() {
  local now=$(date +%s) i
  for (( i = 1; i <= ${#MODELS}; i++ )); do
    if (( QUOTA_UNTIL[i] <= now )); then
      mi=$i
      MODEL=$MODELS[i]
      return 0
    fi
  done
  return 1
}

# Seconds until the first of them is available again.
soonest_reset() {
  local now=$(date +%s) i soonest=$QUOTA_UNTIL[1]
  for (( i = 2; i <= ${#MODELS}; i++ )); do
    (( QUOTA_UNTIL[i] < soonest )) && soonest=$QUOTA_UNTIL[i]
  done
  print $(( soonest > now ? soonest - now + 5 : 60 ))
}

# The gate itself. Narrow on purpose: it says the pool still compiles and its
# data tests still hold, which is not the same as a card being right.
gate_runs() {
  ( cd "$ROOT" \
      && cargo check -p baylee-cards --quiet \
      && cargo test -p baylee-cards --quiet \
      && cargo run -q -p xtask -- validate ) >> "$1" 2>&1
}

# The model is passed in rather than read from $MODEL: a card sits in the
# pending chunk until the gate runs, and a quota met in between will have
# moved $MODEL on. The commit has to name whichever model actually wrote it,
# because that is the one thing a reviewer of an unreviewed card can use.
commit_card() {
  git -C "$ROOT" add "$1"
  git -C "$ROOT" commit -q -m "feat(cards): $2

Implemented by $3 through tools/cardbatch. Unreviewed: the narrow gate
says it compiles and the data tests pass, which is not the same as the card
being right.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
}

# Gates everything accepted since the last flush, then commits it one card per
# commit.
#
# The happy path is a single gate run for up to $GATE_EVERY cards. When it
# fails, the chunk cannot say which card broke it — so it replays: every
# pending card is reverted, then restored one at a time from $HOLD with a gate
# after each. That is the old per-card cost, but only for a chunk that really
# does contain a bad card, and no good card is lost to its neighbour.
# The refusals collected since the last banking, appended to the tracked
# ledger and committed.
#
# Committed rather than merely written, and at every chunk boundary rather than
# once at the end. `data/card-refusals.tsv` is tracked, so an uncommitted
# change to it would show up in the next card's `git status` as a file the
# model touched, and be reverted as a stray. Once it is a commit, nothing in
# this script can reach it — and a run killed halfway (which is how the first
# one ended) no longer takes every refusal it recorded with it.
bank_refusals() {
  [ -s "$REFUSALS" ] || return
  cat "$REFUSALS" >> "$LEDGER"
  git -C "$ROOT" add "${LEDGER#$ROOT/}"
  git -C "$ROOT" commit -q -m "chore(cards): $(wc -l < "$REFUSALS" | tr -d ' ') refusal(s) from a card batch

Every one of these was reverted, so the cards are still generated stubs. The
rows say why: \`cannot_say\` names what the DSL cannot express and
\`nearest_existing\` the closest variant that does exist.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
  echo "  ledger: $(wc -l < "$REFUSALS" | tr -d ' ') row(s) committed"
  : > "$REFUSALS"
  push_home
}

flush_gate() {
  bank_refusals
  [ ${#PENDING} -eq 0 ] && return
  local log=$LOG/gate-$(date +%H%M%S).log
  local i card name slug
  if gate_runs "$log"; then
    for (( i = 1; i <= ${#PENDING}; i++ )); do
      commit_card "${PENDING[$i]}" "${PENDING_NAMES[$i]}" "${PENDING_MODELS[$i]}"
    done
    echo "  gate ok — ${#PENDING} card(s) committed"
    push_home
  else
    echo "  gate failed for the chunk — replaying ${#PENDING} card(s) one at a time"
    git -C "$ROOT" checkout -- "${PENDING[@]}"
    for (( i = 1; i <= ${#PENDING}; i++ )); do
      card=${PENDING[$i]}
      name=${PENDING_NAMES[$i]}
      wrote=${PENDING_MODELS[$i]}
      slug=${${card:t}:r}
      cp "$HOLD/$slug.rs" "$ROOT/$card"
      if gate_runs "$LOG/$slug.gate"; then
        commit_card "$card" "$name" "$wrote"
        echo "    $slug kept"
      else
        git -C "$ROOT" checkout -- "$card"
        python3 "$HERE/verdict.py" "$LOG/$slug.json" row "$slug" "$name" gate-failed \
          >> "$REFUSALS"
        echo "    $slug reverted — see $LOG/$slug.gate"
      fi
    done
    push_home
  fi
  PENDING=()
  PENDING_NAMES=()
  PENDING_MODELS=()
}
for dir in "$PKGS"/*(/); do
  [ $done_n -ge $COUNT ] && break
  slug=${dir:t}
  card=crates/baylee-cards/src/cards/$slug.rs
  # Already finished by an earlier run, or by codegen's own readers.
  grep -q '// GENERATED STUB' "$ROOT/$card" 2>/dev/null || continue
  # Already declined by an earlier run. A model call is the scarce resource
  # here — roughly fifteen per model before the daily quota — and re-asking a
  # card the DSL provably cannot express spends one to record a refusal that is
  # already in the ledger. Extend the DSL and clear the rows for what it now
  # covers, or set RETRY_REFUSED=1 to ask them all again.
  if [ "${RETRY_REFUSED:-0}" != 1 ] && cut -f1 "$LEDGER" | grep -qx -- "$slug"; then
    continue
  fi
  done_n=$((done_n+1))
  name=$(sed -n '1s/^\/\/! \([^—]*\).*/\1/p' "$ROOT/$card" | sed 's/ *$//')
  echo "=== [$done_n/$COUNT] $slug — $name"

  verdict=$LOG/$slug.json
  # The card is attempted until it is actually attempted. A batch that runs
  # overnight will meet the daily quota, and `agy` answers that in three
  # seconds with `status: ERROR` — which the loop below would have recorded as
  # a refusal and moved on from. The first run to hit it burned 202 of its 459
  # cards that way in under ten minutes, refusing every one of them without
  # ever asking the model. So: wait for the reset the message names, and try
  # the same card again.
  while :; do
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

    grep -q 'quota reached' "$verdict" 2>/dev/null || break
    wait_s=$(quota_seconds "$verdict")
    QUOTA_UNTIL[$mi]=$(( $(date +%s) + wait_s ))
    echo "  $MODEL: quota reached, back in ${wait_s}s"
    # The quota is counted per model, so the first answer is another model and
    # not a nap. Sleeping was the whole strategy while there was one name in
    # $MODELS, and it meant a batch spent more of the night waiting than
    # working.
    if pick_model; then
      echo "  → $MODEL takes the same card"
      continue
    fi
    # Every one of them is spent. Whatever is already accepted goes in first:
    # a chunk left sitting uncommitted for two hours is a chunk that anything
    # at all can lose.
    flush_gate
    wait_s=$(soonest_reset)
    echo "  every model is out — sleeping ${wait_s}s"
    sleep "$wait_s"
    pick_model || true
  done

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
    FAIL_STREAK=$((FAIL_STREAK+1))
  else
    FAIL_STREAK=0
  fi

  # What the model actually left behind. The pending cards are legitimately
  # dirty now, so "it touched only its own file" is a set difference rather
  # than the string compare it used to be.
  typeset -a changed stray
  changed=(${(f)"$(git -C "$ROOT" status --porcelain | awk '{print $2}')"})
  if [ "$verdict_status" = "implemented" ] || [ "$verdict_status" = "partial" ]; then
    stray=()
    for f in $changed; do
      [ "$f" = "$card" ] && continue
      (( ${PENDING[(Ie)$f]} )) && continue
      stray+=$f
    done
    if [ ${#stray} -gt 0 ]; then
      echo "  touched more than its own file: $stray — reverting those"
      git -C "$ROOT" checkout -- $stray 2>/dev/null
      git -C "$ROOT" clean -fd -q -- $stray 2>/dev/null
      verdict_status=refused
      outcome=stray-edits
    elif (( ! ${changed[(Ie)$card]} )); then
      # Nothing to keep: the model reported a card it never wrote. Its own
      # reason, and not the same thing as declining one.
      echo "  reported a card it never wrote — recording"
      verdict_status=refused
      outcome=no-edit
      NO_EDIT_STREAK=$((NO_EDIT_STREAK+1))
    else
      cp "$ROOT/$card" "$HOLD/$slug.rs"
      PENDING+=$card
      PENDING_NAMES+=$name
      PENDING_MODELS+=$MODEL
      NO_EDIT_STREAK=0
      echo "  accepted — ${#PENDING}/$GATE_EVERY waiting for the gate"
    fi
  fi

  if [ "$verdict_status" = "refused" ]; then
    # Revert everything except what is waiting for the gate. A declined card
    # may still have left a half-written file behind, and that must go; the
    # chunk's accepted cards must not.
    for f in $changed; do
      (( ${PENDING[(Ie)$f]} )) && continue
      git -C "$ROOT" checkout -- "$f" 2>/dev/null || rm -f "$ROOT/$f"
    done
    python3 "$HERE/verdict.py" "$verdict" row "$slug" "$name" "$outcome" >> "$REFUSALS"
    echo "  $outcome — recorded"
  fi

  [ ${#PENDING} -ge $GATE_EVERY ] && flush_gate

  if [ $NO_EDIT_STREAK -ge $NO_EDIT_LIMIT ]; then
    echo "$NO_EDIT_STREAK cards in a row reported without an edit — stopping."
    echo "That is the prompt, not the pool. Fix it before spending the rest."
    break
  fi

  if [ $FAIL_STREAK -ge $FAIL_LIMIT ]; then
    echo "$FAIL_STREAK cards in a row and the model never answered — stopping."
    echo "Not the quota (that waits). Look at $LOG/$slug.json."
    break
  fi
done

flush_gate

echo "done: $done_n card(s) attempted"
