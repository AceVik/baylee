# Handover — the client, on branch `gpt-6-astra`

Written 2026-09-17 for GPT-6 Astra, by the Claude session that works on the
engine. It is **branch-local scaffolding**: delete it when this branch merges.

`CLAUDE.md` and `AGENTS.md` are the standing contracts and stay authoritative.
This file is the part that is only true right now — what is already verified,
what is yours, and how to get a table on screen without rediscovering the
three traps that cost this session time.

---

## 1. Who owns what, and why it matters

Two worktrees of **one** repository, one checkout each:

| Tree | Branch | Owner | Subject |
|---|---|---|---|
| `~/Projects/baylee` | `gpt-6-astra` | you + the repo owner | the client |
| `~/Projects/baylee-client` | `main` | the Claude session | engine, cards, codegen |

They share one `.git` and one remote, and **separate `target/` directories**,
so neither blocks the other on the cargo lock.

**Stay inside the client.** That is the whole reason this split exists:

- Yours: `crates/baylee-client/`, `crates/baylee-client-core/`,
  `crates/baylee-view/`, `docs/client.md`, `crates/baylee-client/CLAUDE.md`.
- Not yours: `crates/baylee-engine/`, `crates/baylee-cards*/`, `xtask/`,
  `crates/baylee-gateway/`, `crates/baylee-gamehost/`, `docs/card-dsl.md`,
  `docs/engine-*.md`.

If something outside your half looks wrong, **write it down in this file and
say so** rather than fixing it. A one-line engine fix here and a one-line
engine fix there is a merge conflict in a file neither branch can win.

Nothing is pushed. `main` is 74 commits ahead of `origin/main`, and this
branch sits beside it at `64e8095c`.

---

## 2. What is already verified — do not go fixing it

All of this was measured on this machine, on this commit, today. If one of
them fails for you, that is news; it is not a pre-existing mess to tidy.

**Build and gates**

- `cargo fmt --all` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo test --workspace --all-targets` — **43 suites, all green.**
- `cargo build --workspace --bins` — all five binaries link
  (`baylee-gateway`, `baylee-agent`, `baylee-engine-server`, `baylee-client`,
  `baylee-catalog`).
- CI's wasm job, run locally and green:
  `cargo check -p baylee-core -p baylee-protocol -p baylee-view
  -p baylee-client-core -p baylee-client --target wasm32-unknown-unknown`
  (~10 min cold — it compiles Bevy a second time).
- `xtask validate` — 1536 cards conform. `xtask cr-check` — 2122 citations.
  `xtask codegen --check` — up to date.

One warning is expected and is **not** yours to silence: the linker says
`__eh_frame section too large (max 16MB)` when it links `baylee-client`. It
is a debug-profile size artefact of a Bevy binary on macOS.

**Runtime, end to end**

Gateway → agent → a spawned `baylee-engine-server` → a networked client:
the client attached, drew a hand with real card art out of the catalog,
answered the mulligan in German, and reached turn 1 first main phase with two
legal land drops offered. The stack is fine.

---

## 3. Bringing it up

**PostgreSQL is already running and already ingested** — do not re-ingest.
It is a `postgres:18-alpine` container under project `baylee`, up for days,
holding **542 177 printings**. Checks:

```bash
docker compose -p baylee ps
docker compose -p baylee exec -T postgres psql -U baylee -d baylee -c "select count(*) from cards"
```

`psql` is not on the PATH — always go through `docker compose … exec`.

`.env` exists in this tree and carries `BAYLEE_GATEWAY` and `DATABASE_URL`.
Export the database URL for anything that touches it:

```bash
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
```

### The fastest path: a solo duel, no servers at all

```bash
cargo run -p baylee-client
```

That is `LocalHost` — a duel in process against the house AI from
`data/acceptance-decks.txt`. No gateway, no account, no agent. It is the right
loop for anything about drawing, layout, motion or input.

### The full stack, when you need the real wire

Three terminals, in this order:

```bash
# 1 — gateway (needs DATABASE_URL; the token is a shared secret, any hex will do)
export BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32)
RUST_LOG=info ./target/debug/baylee-gateway

# 2 — agent, with the SAME token
export BAYLEE_AGENT_TOKEN=<the same value>
export BAYLEE_GATEWAY=http://127.0.0.1:28766
RUST_LOG=info ./target/debug/baylee-agent

# 3 — a seated table that launches its own client
export BAYLEE_GATEWAY=http://127.0.0.1:28766
cargo run -p xtask -- dev-table --seats 2 --ai sharp --play
```

The gateway has **no `/health` route** — do not wait on one. It is up when it
logs `baylee-gateway listening port=28766`; `lsof -nP -iTCP -sTCP:LISTEN` is
the second opinion.

> **Trap, measured today.** Use `--play`, or have the client binary *already
> built* before you create the table. Without `--play` this session created a
> table, then spent three minutes building the client, and by the time it
> connected the engine's decision clock had run the dev seat out — exactly
> **60 s** from `engine attached` to `decision timed out seat=0`, and the game
> was over with the AI as the last player standing. It looks like a broken
> stack and is a stopwatch.

`dev-table` skips the lobby's sign-in and deck-picking *screens* and nothing
else: the account (`dev@baylee.local`, reused across runs), the deck, the room
and every AI chair are made through the gateway's own HTTP routes, and the
game runs over the same sockets as any other. `--teams 1,1,2` makes a 2v1 and
needs three chairs or more.

### Driving the client while its window is in the background

This is the tool that makes a render claim provable, and there is a skill
for it at `.claude/skills/dev-control/` worth reading in full.

```bash
BAYLEE_DEV_CONTROL=28770 cargo run -p baylee-client --features dev-control
until curl -sf localhost:28770/health >/dev/null; do sleep 1; done
curl -s localhost:28770/state | python3 -m json.tool | head -40
curl -s -XPOST localhost:28770/screenshot -d '{"path":"/tmp/table.png"}'
```

Routes: `/health`, `/state`, `/key`, `/text`, `/pointer`, `/scroll`,
`/screenshot`, plus `/timescale`, `/pause`, `/step` for photographing an
animation. Five things that go wrong are written out in the skill; the two
that bite first are that **a click is three frames** (the route answers after
the release) and that **`/pointer` takes logical coordinates while a
screenshot is physical** — `/health` reports `scale` so the ratio is read and
not guessed.

> **Trap.** `cargo test` rebuilds `target/debug/baylee-client` *without*
> `dev-control`, silently. Before starting a drivable client after a test run:
> `strings target/debug/baylee-client | grep -c BAYLEE_DEV_CONTROL` — it is 2
> with the feature and 0 without. (It is 0 right now: this session put the
> plain binary back so your first `cargo run` is the ordinary one.)

### Editing a shader while the game runs

```bash
BAYLEE_DEV_CONTROL=28770 BEVY_ASSET_ROOT=$PWD \
    cargo run -p baylee-client --features dev-control,dev-reload
```

Neither variable is optional. Without `BEVY_ASSET_ROOT` the watcher looks in
the wrong place and reloads nothing; without `BAYLEE_DEV_CONTROL` you cannot
stop the picture while you work on it.

### The browser build

`trunk serve index.html --release` from `crates/baylee-client/`. **Always
`--release`** — a dev-profile wasm is ~350 MB against ~36 MB. It renders
through WebGPU. Note that nobody has actually *played* the browser build; it
is built and never used, which is an open question rather than a supported
path.

---

## 4. Where things are

```
crates/baylee-client-core/   the whole client brain — layout, board model,
                             interaction state machine, mana planner, i18n,
                             lobby. Knows no renderer. Most client tests.
crates/baylee-client/        Bevy. The only crate that needs a GPU.
crates/baylee-view/          the wire view: projected characteristics only,
                             no rules kernel. `VIEW_VERSION` lives here.
```

**New behaviour belongs in `baylee-client-core`**, where it runs in
milliseconds and in CI. `baylee-client` draws what core decided.

Normative reading, in the order it pays off:

- `crates/baylee-client/CLAUDE.md` — loads automatically for work under that
  directory. Note it does **not** load for `baylee-client-core/`; open it by
  hand, because most of what it describes is decided there.
- `docs/client.md` — 4547 lines and the real reference. Useful section
  anchors: *The table itself* (75), *Which ability is on the stack* (528),
  *Images and memory* (946), *The card surface* (1201), *Motion* (1487),
  *Hosts* (1680), *The lobby* (1748), *The deck builder* (1865), *Tapping
  lands for a spell* (1987), *Every seat's bar* (2475), *The sounds* (3184),
  *The zone browser* (3392), *Settings* (3869), *The interface's own words*
  (3920), *In the browser* (4124), *On a phone* (4158), *Driving the client
  without its window* (4199).
- `docs/protocol.md` — "The gateway runs no rules", if a question is about
  where a frame comes from.
- `.claude/skills/bevy/` — this workspace is on **Bevy 0.19**, and 0.19 broke
  a lot of older habits: resources are components, the render graph is gone
  (render passes are systems in the `Core3d`/`Core2d` schedules), `FontSize`
  is an enum and not an `f32`, `bsn!` is the scene syntax. Treat any Bevy
  knowledge older than 0.19 as a hypothesis and check `docs.rs/bevy/0.19.1`.

---

## 5. Five invariants that look like bugs and are not

1. **The stage has no lights at all**, and the camera carries
   `Tonemapping::None`. Everything on the table is `unlit`. Scene lighting on
   card art destroys colour identity. Do not "fix the lighting".
2. **Nothing on the table is positioned directly.** `table::sync_scene`
   writes a `Motion` target and `table::glide` moves the card. A repacked
   lane, a tap, a hover and a card entering play all animate through that one
   door. Writing a `Transform` yourself desynchronises the scene from the
   board model.
3. **There is no text on the 3D table.** Numerals are a 4×6 stencil in the
   shader. If you need a glyph, that is a design decision, not an oversight.
4. **Input asks actions, never keys.** The account's `Keymap` resolves them
   in `keys.rs`; `docs/keyboard-map.md` is the list.
5. **Every shader stays inside the old GL budget** — uniforms only, no
   storage buffers, no texture arrays, no `bitCount` — so `webgl2` remains a
   one-word fallback. Reaching past it is allowed, but it is a decision the
   commit has to state, because it is the commit that closes the fallback.

And one anti-cheat invariant you cannot work around: hidden information has
no field to leak through. Libraries are counts, another seat's hand is a
count, a face-down permanent's `card` is `None` for anyone not entitled to
look. If the client seems to lack data, the answer is usually a **projection**
in `baylee-view`, not a wider view.

---

## 6. Known open client defects — real work, already diagnosed

**The mana planner picks the ability with the most mana, ignoring its cost.**
`crates/baylee-client/src/manasources.rs::sources` folds a permanent into one
source ("a permanent taps once") and sorts `b.amount.cmp(&a.amount)`, then
`b.colors.len().cmp(&a.colors.len())`. The ability's *cost* never enters the
choice. Measured: Havenwood Battleground prints `{T}: Add {G}` and
`{T}, Sacrifice this land: Add {G}{G}`, so the planner **sacrifices the land**
the moment it taps it; the Vivid lands burn a charge counter when plain red
would have done. The repair needs a cost weight on `Source` — pays more than
the tap: `SacrificeSelf`, `ExileSelf`, `ReturnSelfToHand`, `RemoveCounterSelf`,
`PayLife` — sorted *last*, after amount and colours. Test both ways: a board
with Havenwood and a need of `{G}` must not sacrifice it, and the same board
needing `{G}{G}` very much may. The engine is not involved; it offers whatever
`can_afford` allows.

A second, older one next door: `manaread::mana_shape` reads an ability as mana
only when its effect list is **exactly one** `AddMana`. Every painland and the
six counter-sacrifice lands have two effects and drop out of the plan. The
module header says that is deliberate ("an ability that also does something
else is the player's decision"), so it is a UX question rather than a bug —
but it is why a new land suddenly "isn't planned for", and that looks like a
fault.

**Combat has no input path.** `baylee-client-core::interaction::Interaction`
offers `declare_attacker` and `declare_blocker`, the combat *lines* are drawn
and tested (`crates/baylee-client/src/table/combat_tests.rs`) — and the only
production code in the whole client that sends `PlayerAction::DeclareAttackers`
is `lib.rs:1377`, which sends `vec![]` from the automation path. Verified
today by grep over all 113 client source files. So attacks and blocks are
answered with empty lists, always.

This reads as "combat is implemented" because client-core can do it and has
tests. The gap is one layer up, in the Bevy input layer, and you will not find
it by reading `interaction.rs`. The candidate lists are **not** a client
problem: `Pending::ChooseAttackers` carries `attackers` + `defenders`, and
`Pending::ChooseBlockers` carries one `BlockOption { blocker, attackers }` per
blocker. What is missing is the UI — attacker pick (creature from `attackers`,
then a target from `defenders`), then blocker assignment along the
`BlockOption` pairs.

Beware the shape of the test when you do it: a test that hand-builds a
`PlayerAction` or an `Interaction` passes happily while there is no button on
screen. That is exactly how this gap stayed invisible. A client test has to
answer the way a player does.

---

## 7. The owner's UX backlog lives outside the repo

`~/.claude/projects/-Users-viktor-Projects-baylee/ux-backlog.md` — 17 items
given 2026-09-10, deliberately outside the repo so it survives a restart, with
dev-control helper scripts (`q.sh`, `play.sh`, `goto.sh`, `turn.sh`,
`watch-exit.sh`) beside it. **Read it before picking work**; it is the live
priority and is not derivable from the code.

It opens with **four questions addressed to the owner** that still block work:
item 1's "beside it" (where a copy shows its original), item 11's defender
stillness and per-card animation phase, and item 14's "to meet them" for a
blocker. Those are looks, not correctness — ask, do not guess.

Done as of 2026-09-10: 2, 3, 5, 8, 9, 10, 15, 16, 17 and item 1 bar its rider.
In progress: 4. Remaining: 6, 7, 11, 12, 13, 14.

---

## 8. Housekeeping that will bite

**Never commit these.** They are untracked on purpose and some are
deliberately *not* gitignored:

- `gateway-store.json` — account credentials.
- `gateway-store.json.imported` — the same, after a first start moved it aside.
- `client6.log`, and any other client log.
- `.junie/`.

Stage explicitly — `git add <file> <file>`, never `git add -A`.

**Gates while you work.** `./scripts/gate-rules.sh` is fmt + clippy + tests
for everything *except* `baylee-client`, so for client work it is exactly the
wrong filter — run the real thing:

```bash
cargo fmt --all
cargo clippy -p baylee-client -p baylee-client-core -p baylee-view --all-targets -- -D warnings
cargo test -p baylee-client -p baylee-client-core -p baylee-view
```

and the full `cargo test --workspace --all-targets` before you call something
finished. **One cargo command at a time** — two runs block each other on the
`target/` lock, and a scope flip (`-p x` then `--workspace`) rebuilds Bevy.

**Never run `cargo run -p xtask -- codegen`.** It rewrites every card file
carrying the stub marker, it needs an external GPL corpus that is not
vendored, and the engine session may have a card batch in flight. It is not
client work under any circumstance.

**Bump `VIEW_VERSION`** (in `crates/baylee-view/src/lib.rs` — read it there,
this kind of file has been stale on it twice) on any breaking view change, so
a client refuses a host it cannot render. Gamehost and client tests assert it.

**This machine's shell is not stock.** `du` is `dust` (use `command du`),
`ls` is `eza` (use `/bin/ls` for `-t`), `find` is `bfs`, `grep` is a shell
function that misbehaves in pipes (use `/usr/bin/grep`, and it has no `-P`
here), and `timeout` is not installed.

---

## 9. What changed on the engine side today

Two commits, both on `main` and both already in this branch's history, so a
client test that suddenly cares about counters has a reason:

- `96714420` — counters placed **as a permanent enters** used to land behind
  the layer projection that the state-based actions read, so a printed 0/0
  arriving under a +1/+1 counter was put into a graveyard before anybody was
  asked anything. One `continue` in the machine loop.
- `64e8095c` — `EnterModifier::WithCounters` carries an `Amount`, so
  "this creature enters with X +1/+1 counters on it" is expressible
  (CR 107.3m). Walking Ballista is `Coverage::Implemented`.

Neither touches the view, the protocol or the client.
