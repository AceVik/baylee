# Dictionary

Five sessions and one owner work on this repository at the same time. This file
exists so that a word means one thing to all of them.

It is **not** a glossary of Magic, and not a summary of `CLAUDE.md`. An entry
earns its place by being **ambiguous**: a word this repo uses for two different
things, or a word whose everyday meaning is not the meaning here. If a term is
obvious from the code, it does not belong here.

**Who maintains it:** the PM session curates and commits. Any session may
propose an entry by message — say the word, the two things it could mean, and
which one is meant where. An entry that turned out to be wrong is corrected, not
appended to.

---

## The words this repo uses for two different things

These are the ones that have actually caused a misunderstanding, or are one
sentence away from causing one.

### index

Three of them, and none is a row number.

- **`CardIndex`** — the append-only ledger id that names *every card there is*
  (33 694 rows), not this pool. What saved decks and replays store. Spelled as a
  constant: `index::MOX_OPAL`.
- **`SubtypeId`** — an id from a compiled, **append-only** table. It *used* to
  be a running index into one sorted range partitioned by kind, and renumbered
  every other kind whenever a creature type was added; since #43 the generated
  table is its own ledger and no id ever moves. What is still true is that an
  id minted by a newer build is unknown to an older one — `subtypes::kind` and
  `subtypes::name` both answer `None` — so a stored id is only ever as good as
  the build that reads it.
- **ability index** — the `index` in `AbilityRef { card, index }`. Reserved ones
  (`SPELL`, `ENTERS`, `choice::GRANTED_ABILITY`, …) count down from `u32::MAX`.

Say which one. "The index" alone is a question, not a fact.

### projection

- **view projection** — the *characteristics* `baylee-view` carries after the
  layer system has run (P/T after anthems, a clone's name). A client cannot run
  layers, so the host projects for it.
- **catalog projection** — the `card_search` table, one row per oracle face,
  rebuilt wholesale by `Catalog::project()`. Nothing to do with the layer
  system.

### ledger

- **the card ledger** — `crates/baylee-cards-index`, one `Row` per card,
  assigns `CardIndex`. Append-only.
- **the token ledger** — `tokenledger::assign`, which gives a token its id in
  `generated_tokens::ALL` and refuses a name two definitions claim.

### lock

- **the cargo lock** — `/Users/viktor/.baylee-locks/with-cargo-lock.sh`, a
  *directory* lock held across all five trees so two heavy cargo runs do not
  fight over the CPU. This is the one that matters to a session.
- **`Cargo.lock`** — the dependency file.
- **cargo's own lock** — what a second `cargo` in the *same* `target/` blocks
  on. Separate trees already avoid this one; they do not avoid the first.

### agent

- **`baylee-agent`** — the process that starts one engine per game and talks to
  the gateway over `/agent/ws`.
- **an agent session** — one of us. A Claude session, or Astra (Codex).

Never write "the agent" in a commit message about a session.

### AI

- **the house AI** — `baylee-ai`, `HeuristicAgent`, what plays a seat.
- **us** — the sessions. Say "session", never "AI", in anything committed.

### gate

- **the rules gate** — `./scripts/gate-rules.sh`: fmt + `cargo lint-rules` +
  `cargo test-rules`, which is the workspace **minus `baylee-client`**. A filter
  for working, never a replacement.
- **the full gate** — the same over the whole workspace, client included. The
  one that runs before a push, because the rules gate cannot see the client.

### tree

Always a **git worktree**, never a data structure. There are five; see
*Territories* below.

### bar, ledge, drawer, tray

Four panels in the client, and German has one word for two of the pairs. Say
the precise one even when speaking German.

- **bar** — the **seat** bar, `hud/seatbar.rs`, gated by `BarRevision`. Never
  anything else.
- **ledge** (*Leiste*) — the merged hand-and-question bar, `hud/ledge.rs`.
  #10's title says "action bar" and means the ledge; the title is what is
  wrong, not the crate.
- **drawer** (*Lade*) — what grows out of the ledge, `hud/ledge/drawer.rs`.
- **tray** — the put-down dialog, `hud/tray.rs`, gated by `TrayRevision`. A
  different panel from the drawer, and the German for both is one word away.

**shelf, and why "ledge" names two things on purpose.** `docs/client.md` uses
*shelf* 79 times and *ledge* 34, and each names two different surfaces — the
disambiguator is the section heading, which is exactly what a reader grepping
for the word does not have. It looks like a collision to clean up. It is not:

- `tabletop::MAT_LEDGE` is the **mat's** shoulder, the band a seat bar is
  written along, and **the shelf** is the defined sum `MAT_MARGIN + MAT_LEDGE`
  (`LEDGE_FRAC`, `tabletop.rs:506`) — geometry, with a measured depth.
- `hud::ledge` took that name **deliberately**: "It is named for
  `tabletop::MAT_LEDGE`… The hand zone gets the same shoulder, at its top"
  (`hud/ledge.rs:1-13`), and its own doc says it "holds the shelf and the
  middle of it".

So one word names one *feature* on two surfaces, which is the design saying
something true. Renaming either half would delete an analogy the code states
in two places.

**The rule is therefore a qualifier, not a rename: never a bare "shelf" or
"ledge" where both surfaces are in play.** Write *the mat's shelf* and *the
hand's shelf*, *the mat ledge* and *the HUD ledge*. A sentence that needs its
section heading to be read correctly is a sentence missing a word.

And the qualifier is only half of it. Applying the rule to `docs/client.md`
took **eight edits out of 107 occurrences** — 79 uses were already unambiguous
from what is cued within three lines, and qualifying those would have been
noise. What was genuinely wrong split in two, and only one kind is a missing
word:

- **A bare word for the surface the section is *not* about** — the zone
  browser's `LedgeRevision` rebuilding the hand's shelf among 54 neighbouring
  uses meaning the mat's. A qualifier fixes it.
- **A paragraph alternating both words for *one* surface**, a line apart:
  "it reports both *shelves* … the number that decides the *ledge*". That
  reads as two things where there is one, and **no qualifier repairs it** —
  the fix is to stop alternating.

So: qualify where both surfaces are reachable, pick one word and keep it where
only one is, and put the rule at the head of the section rather than a
qualifier on every line inside it. The pass is the cheap half; the next writer
is the expensive one.

---

## Terms of art you are expected to know

### the three card sets

- **the pool** — the 1365 cards this repo compiles (`crates/baylee-cards`).
- **the corpus** — the 33 694 cards the ledger numbers, scanned by
  `baylee-catalog corpus`. A superset of the pool.
- **the reference** — the external, GPL-licensed card-script corpus. Read as an
  automated lookup, **never copied into this repo**. Not vendored, found three
  ways (`--scripts`, `BAYLEE_CARD_SCRIPTS`, a `cardsfolder` near the parent).

A fourth, separate thing: **the catalog** — PostgreSQL card *text* for the
client, optional, unrelated to all three.

### the three ownership states of a card file

- **stub** — `stubgen::STUB_MARKER`. Nobody has finished it. Rewritten on every
  codegen run.
- **machine-owned** — `stubgen::OWNED_MARKER`. A reader wrote it in full.
  Rewritten on every codegen run. **Never patch one**; fix the reader.
- **hand-written** — neither marker. Never touched by codegen.

`xtask adopt --name "<card>"` is the one door from machine-owned to
hand-written, and it is one-way.

### the honest-stub rule

One unread clause and the card is refused. A generated `Coverage::Implemented`
therefore means exactly what a hand-written one means, because the deckbuilder
offers `Implemented` cards as playable. Getting this backwards is worse than
generating nothing.

### first refusal

`transcode-report` lists a script under the **first** thing that refused it. So
closing one cause moves every card that had it to whatever refuses it next. The
cause going to zero measures the rule; the cards finished measures the residue.
They are different numbers and a commit says which one it was aiming at.

### `--stubs`

`transcode-report` ranks the whole reference by default, which measures the
**DSL**. `transcode-report --stubs` ranks only this pool's unfinished cards,
which measures **cards shipped**. The two orders disagree sharply. Rank
corpus-wide to grow the DSL; rank `--stubs` to finish cards.

### two-phase

`codegen` writes some tables from the *compiled* pool, so a card added by one
run gets its row from the next. Run it twice. Only `codegen --check` sees the
gap.

### seat vs. chair

A chair and whoever answers for it are two questions. `SeatKind` is
`Human · Ai · Driven · StandIn`; `answers_over_socket()` is the question almost
everything actually wants. A held chair is **not** relabelled an AI —
`SeatIdentity.away` says so instead.

### Revier (territory)

The crates a session owns. Overlap is **announced, not discovered**. In someone
else's Revier the owner of it is the expert, and a measurement beats seniority
in both directions.

---

## Territories

| Session | Tree | Branch | Owns |
|---|---|---|---|
| PM | `/Users/viktor/Projects/baylee` | `main` | board, integration, this file, the cross-cutting `docs/` |
| engine | `…/baylee-engine` | `engine` | `baylee-engine`, `baylee-core`, `baylee-cards`, `-cards-dsl`, `-cards-codegen`, `-cards-index`, `baylee-gamehost`, `baylee-engine-server`, `xtask` |
| client | `…/baylee-client` | `client` | `baylee-client`, `baylee-client-core`, `baylee-client-android`, `scripts/mobile` |
| gateway | `…/baylee-gateway` | `gateway` | `baylee-gateway`, `baylee-agent`, `baylee-protocol`, `baylee-db`, `baylee-catalog` |
| ai | `…/baylee-ai` | `ai` | `baylee-ai`, house-AI scouting, `docs/house-ai.md` |

Two crates belong to nobody and are **announced before they are changed**:

- **`baylee-view`** — `VIEW_VERSION` is asserted in gamehost *and* client tests,
  so a breaking change reaches engine, client and gateway at once.
- **`baylee-build`** — the compile-time build stamp every binary reads.

**One branch at a time reaches for the next `VIEW_VERSION`.** Not because the
conflict is expensive — it is one line — but because of what a clean merge
looks like when two branches both ship "22 → 23": the numbers now agree and
the two view *shapes* do not, and the assertion in gamehost and client tests
passes precisely because it only ever compares the number. A client would then
refuse nothing and render a host it cannot read. So the sequence is held on
whichever branch already owns it.

That is a rule about **numbers**, not about time: a session claims the next
free one from the PM before it writes the constant, and two branches carrying
23 and 24 may be open at once.

What must hold is checked at the merge and nowhere else: **the branch's
constant is exactly main's plus one.** A claim is therefore provisional —
whoever rebases second finds main already at their number and takes the next
one, which is a one-line change in a rebase they are doing anyway. Nobody
waits on anybody, and the PM runs the comparison before every ff-merge:

```bash
git show main:crates/baylee-view/src/lib.rs           | grep -o 'VIEW_VERSION: u32 = [0-9]*'
git show origin/<branch>:crates/baylee-view/src/lib.rs | grep -o 'VIEW_VERSION: u32 = [0-9]*'
```

A session reads the constant out of
`crates/baylee-view/src/lib.rs` when it starts rather than out of the ticket
it was written on; this repo has been stale on that number three times.

**A document follows its subject, not this table.** `docs/` as a whole is the
PM's, but a file that is normative for one Revier belongs to that Revier's
session and is committed on its branch with the change it describes —
`docs/client.md` with the client, `docs/house-ai.md` with the AI,
`docs/protocol.md` with the gateway, `docs/engine-internals.md` and
`docs/card-dsl.md` with the engine. Prose written in one branch and committed
in another drifts from the code between the two commits, and this repo has
paid for that. What stays the PM's is what belongs to no Revier: this file,
the roadmap, and anything that describes how the team works.

`scripts/llm/` is card volume and belongs to the engine session, which plans it
in cards (DeepSeek, billed per token) and in minutes (Gemini, a time quota).
**No lane runs cargo**, and no `xtask codegen` runs while a batch is in flight.

---

## German ↔ English

The owner writes German; the repo, the board and every message between sessions
are English. These are the pairs that are not a dictionary lookup.

| Deutsch | English | Note |
|---|---|---|
| Baum | worktree | never "tree" as in data structure |
| Revier | territory | which crates a session owns |
| Gate | gate | untranslated; say *rules* or *full* |
| Karte | card | |
| Kartenmasse | card volume | the cheap LLM lanes, not a session |
| Spur | lane | `scripts/llm/`, one script per (model, job) |
| Briefkasten | mailbox | `/tmp/agentbus`, the Codex bridge |
| Sitzung | session | |
| Zug | turn | one agent turn, not a Magic turn |
| Stapel | stack | the Magic zone |
| Zone | zone | |
| Hand ablegen / Tray | tray | the client's put-down dialog |
| Schranke | bound | as in "a reader that reports a population" |
| Naht | seam | an architectural boundary that drops a capability |
| Messung | measurement | the thing that beats an argument |
| Befund | finding | what a report produces; not necessarily a defect |
| Rückfahrt | return trip | the second half of a timing rule |
| Gegenprobe | counter-test | the run that must fail against the old code |
| Vorspulen | fast-forward | `merge --ff-only` |
| ungetrackt | untracked | never staged: `.junie/`, `gateway-store.json.imported` |

---

## The two cheap lanes

Two models below this team's own tier are available to **every** session, not
only to the card lane. The ids are read from `scripts/llm/lane.py:46-47` and
never from memory:

- **Gemini 3.8 Flash (high)** — `gemini-3.8-flash-high`, reached as
  `agy -p <prompt> --model <model> --print-timeout <N>m`.
- **DeepSeek Flash (max)** — `deepseek-flash[1m]`, reached over HTTP with
  `DEEPSEEK_API_KEY`, or an opencode login.

**They are scarce in different units, and that is what plans them.** DeepSeek
costs money per token and is cheap, so it is planned in *items*. Gemini costs
no money but a time quota — and that quota **renews weekly**, which makes it
use-it-or-lose-it: an unspent Gemini week is gone, an unspent DeepSeek token
is not. Prefer Gemini for anything bulky while the week is young. Gemini also
makes **images**, which is the one capability neither this team nor DeepSeek
has: reference imagery and mockups for a look, argued over as pictures before
anyone writes a shader.

**What does not change.** Whoever wrote a thing does not write its test — a
card and its test from one model share that model's misreading. No lane runs
`cargo`; the build belongs to the coordinator. No `xtask codegen` runs while a
batch is in flight. A lane does not write a rule, a heuristic or a look: it
produces volume where a mistake is local, and the session holding the
territory does the reading. And it goes nowhere near the gateway's store or
auth surface — accounts, sessions, tokens, argon2id hashes.

## House phrases

Short sentences this team uses as shorthand. Each one is a rule.

- **"Fix the reader, never the card."** One rule wrote hundreds of files.
- **"A measurement beats the lead."** In both directions.
- **"One unread clause and the card is refused."** The honest-stub rule.
- **"Rank the pool, not the corpus."** When the goal is a finished card.
- **"Never restate a default."** Why the card macros exist.
- **"A blocker entry is a sentence, not a subsystem."** Read what the DSL cannot
  *say* before assuming the mechanism is absent.
- **"Overlap is announced, not discovered."** The Revier rule.
- **"Waiting is free."** A blocking lock costs no tokens; a broken gate does.
- **"A head I report is a head I have pushed."** A gate runs against the
  working tree, so green proves nothing about what anyone else can fetch.
  *Committed* and *reachable* are two different claims, and `git ls-remote`
  settles which one you are making — it also beats the integrator's merge
  record, which by construction only knows what has already been taken.
- **"Rebase as the first step of the gate, not of the work."** With four
  branches, main has moved by the time you are green. A gate against a base
  that no longer exists is not evidence.
- **"A doc-only commit rides with the next substantive one."** Every merge
  costs the other branches a rebase and a re-gate, plus whatever that rebase
  disturbs. Twenty-nine lines of prose is a bad trade for that.
- **"Waiting is free" is about tokens, not about time.** Measured on 19.09:
  three gates waited 364 s / 240 s / 180 s in front of the lock against
  142 s / 136 s / 81 s of work. The wait has been longer than the work every
  time anybody has timed it, and it is invisible in any per-run number. #100.
- **"A narrow run is not a short run."** `-p <crate> --lib <filter>` narrows
  what *runs*, never what *builds*: after a rebase it is a full compile of
  that crate's graph. So nothing on a command line predicts what a run will
  cost, and a "quick filtered test" is how a four-minute lock wait happens.
  Hold the lock around the narrowest command and never across a rebase, an
  edit, or a pause to think.
- **"The lock protects tests from load, not the CPU from waste."** Its reason
  is in its own header: three `baylee-engine-server` e2e tests died on 18.09
  because a second run sat beside them. So exempting cheap runs removes
  protection from whoever is gating — and the failure lands in *their* suite,
  looking like their bug.
- **"Verify the peer, then repeat it."** Twice on 19.09 a correct finding
  arrived with an incomplete list behind it, and once a grep of my own was
  spent on a claim it did not support. A finding is repeatable when you have
  run the search yourself; until then it is theirs, not yours.
- **"Does anything read this?" is a question per function, not per field.**
  Three hashes wear two names here, and two of them are live. A no-caller
  result spread over 66 fields reads as a blanket reprieve while the sibling
  is running in every game.
- **"A gate on a dirty tree says nothing about `HEAD`."** Observed on 19.09:
  a full green gate — fmt, clippy, rc 0, 2929 tests — reported for a head
  that did not compile, because the fix was in the working tree and not in
  the commit. Every number was true of what was on disk and true of nothing
  that was committed. `git status` before quoting a gate.
- **"A green gate describes the tree it ran on, not the tree the merge will
  produce."** The same day, the same break, a second cause: the commit was
  gated before the rebase that brought a changed signature into its tree. A
  rebase replays cleanly when no file overlaps — and a call site that no
  longer matches its callee is a break a rebase cannot see. Rebase first,
  then gate.
- **"When the collision is a subsystem rather than a file, read the other
  side's diff."** File-disjointness has waived a re-gate correctly all day.
  It is not enough when both sides touch one subsystem: if the other diff
  only loosens a bound (a raised timeout), the older gate still holds; if it
  moves an assertion or a signature, it does not. The instrument is the
  **dependency graph and not the file list** — the gateway's change listed a
  file under `crates/baylee-client/` and was a comment, while its dev closure
  genuinely compiled a `baylee-client-core` the other side had rewritten. One
  of those two facts decides and the file list is not it.
- **"The field a literal harness hides is the one whose empty value is also a
  legal value."** Narrower than "a struct literal only claims what it sets",
  and the narrowing is the finding: `board: None` fails loudly, because every
  reader bails and the test notices, while `reachable: {}` and
  `owed_plan: None` are answers a working client gives all day — so the test
  passes whatever the feature does, including nothing. Measured over the
  client: 41 of 192 read a wire-filled field with no fill path in reach, and
  most of the 41 are legitimate. Build the seat through the wire and hand it
  to the surface being asked.
- **"A filter written for one of a tool's verbs hides the other."** `cargo`
  says `Compiling` for a crate it builds and `Checking` for one it only
  type-checks, so a grep for `Compiling baylee` reported that a `cargo check`
  had touched neither of the two crates it had just checked. The run was
  green and the filter said nothing happened — which is the same shape as a
  reader answering a question it cannot see, in the one place it is easiest
  to mistake for a result.
- **"It could not have said yes, so its no was worth nothing."** Before
  running an instrument, ask whether it can in principle produce the
  positive result; if it cannot, its negative is not evidence, and a
  green-looking zero is the most expensive kind. Measured on #23: the
  brick wall was first read spectrally, Hann-windowed at its own spatial
  frequency with the neighbouring bin as counter-reading — the shape that
  worked on #81's mat edge — and returned 0.68–1.45 with the neighbour
  consistently *larger* than the signal, on candidates whose courses are
  plainly visible in the sheet. The precondition had not travelled: felt
  is a **stationary texture**, where a spatial frequency means something,
  while a card face is a **layout**, and four hard horizontal steps put a
  large component in every low bin including the wall's own.
- **"An instrument belongs to the signal, not to the ticket."** The same
  measurement changed twice and said so both times. A row-mean was the
  wash's instrument; when the signal moved into joints about a pixel wide,
  a row-mean averaged it away, so the old column had to go rather than be
  carried forward looking comparable. Changing instrument is correct and
  silent continuity is the failure — but the two columns must be marked
  as not comparable, or somebody puts them in one table.
- **"Pick the geometry that survives the shader, not the one that
  scores."** A joint 0.51 px wide made point-sampled and antialiased
  readings disagree by 30%: a third of the number was where the sample
  happened to land. A 7×18 wall measured better and was rejected for a
  5×13 with a 1.8 px joint, where the two agree to 3%. A measurement that
  depends on pixel-centre luck is not a measurement of the effect.
- **"A census whose parts do not sum to its whole has a card in it nobody
  looked at."** Add the breakdown up before relaying it. Observed 19.09:
  a 35-effect census was relayed as 28 + 3 + 2 + 1, and the missing one
  was a real row. The check costs nothing and catches the one shape that
  survives three retellings — a wrong part beside a right total. The
  cheaper half of the same lesson: the census was caught because the test
  interpolates its own counts into the assertion message, so an
  impossible floor made the pool state its figure out loud.
- **"A cut can overstate a positive; it cannot overstate a zero."** When
  a blocker is measured by cutting it out and re-running, the cause reads
  as *nothing* rather than as something implemented, so a real rule would
  still have to claim that line's parameters. The count that comes back
  is therefore a **floor**. The zeros are the sturdier half of the same
  run. Measured on #72: `effect Effect` 22 listed → 12 read, the whole
  `ChangeZone` family 22 listed → 0, and the two cut together → still 12,
  which is what settles it — a family that adds nothing beside the cause
  that works has something else waiting behind every script.
- **"A register may be the sole carrier of a claim only with margin."**
  Redundancy between two registers is the tidiness fault the face rule
  names — *unless* the single register is marginal, in which case it is
  the correct answer and the rule yields to the measurement. What makes
  the difference measurable rather than arguable: a face register's
  strength varies with what it is composited over, where a border
  sheath's does not. And a double register kept for that reason is
  **pinned**, not permanent: a test asserting the margin is still thin
  goes red the day the single register grows up.
- **"A measured count can answer the wrong sentence."** The sequel to
  "a recalled count reads like a measured one", and the more dangerous
  half, because this one has a green test behind it. Measured 19.09:
  `PoolCard::two_faced` is `def.faces.len() > 1` and returns **120**,
  correctly. The row above it said "Transformation: 120 two-faced", and
  three different numbers hide under that phrase — 120 faces compiled in
  this build, **121** cards whose printing has two faces, ~107 with a
  separate back image, which is what both consumers want. Eleven false
  positives (nine adventures and two splits print both halves on one
  physical face) against one false negative. Re-measuring the predicate
  finds none of it: the predicate is right and its **name** is a claim.
  Hold the name against the sentence, and make the cell say its
  instrument.
- **"A documented gap is not a backlog entry."** A hole described
  precisely in the tree is still invisible work: it is on no board, in no
  sprint, and gets found by a player. CR 608.2b was written up in two
  places — `event.rs` on `StackObjectDidNotResolve` ("a check this engine
  does not make yet") and `docs/engine-gaps.md`, with the line the fix
  belongs on — and the first person to notice was the owner, mid-game,
  when his Heroic Intervention did not save the creature. Good gap prose
  feels *more* finished than none, because the thing is understood,
  named and located. Understanding is not scheduling: a gap explained in
  an answer goes on the board before the answer is sent.
- **"An absence is not a message."** An interface that communicates by
  *withholding* something has not communicated. `touch.rs` argued there
  need be no shake, no red and no message because "the card was already
  saying it could not be cast, through a halo it does not wear" — and
  #112 is a player refuting that sentence. He pressed, the card gave way
  and came back heavy, and he reported it as **completely dead**; offered
  three descriptions he chose "nothing at all" over "reacted briefly,
  then sprang back". The missing halo carried nothing, and the heavier
  return was not legible as a response. Before shipping a signal that
  consists of something not being there, say who is supposed to notice
  its absence and against what.
- **"Sending is not landing."** A test that stops at *armed*, or at "we
  put it on the wire", is satisfied by a client that never completes
  anything. Found by a draft that read the pending on the frame a run
  ended and saw the seat still holding priority with mana floating and
  `CastSpell` unanswered. Assert the far end — the engine's next
  question — not the near one.
- **"A cleared path is only a discriminator if the other path is cleared
  too."** A question keyed on *which* of two states the player saw is
  worth nothing while only one of them has been measured, and it reads as
  conclusive either way. Measured on #112: the indigo path was cleared
  end to end and the gold one was not, and the discriminator built on the
  pair looked finished.
- **"A reason assembled from a residual is a guess wearing a reason's
  clothes."** Two arms that positively know their case plus a third that
  means "none of the above" will confidently tell the player the wrong
  thing the first time a fourth cause appears. Either the third arm knows
  something, or it says the vague true thing — never the specific
  plausible one.
- **"A plausible reconciliation is the dangerous outcome of an
  arithmetic check, not the reassuring one."** When a breakdown does not
  sum to its total, the temptation is to find the story that closes the
  gap. Measured 19.09: the tidy story — 361 cards against 362 abilities,
  one card with two kinds — was consistent, mechanical and wrong in three
  numbers at once. The real answer was 361 · 382 · 383, and the breakdown
  had been taken inside an `any(...)`, which short-circuits: the right
  operator for "does this card do X" and the wrong one for "how many".
  A mismatch sends you to the data, never to an explanation.
