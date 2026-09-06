# The game log, and the seat that stepped away

Two designs, neither built. Written down because the decision was to build
them *later*, and a design that lives only in a conversation is a design that
has to be had again.

## Why a log at all

The report that started it: "I pressed ok without reading, and suddenly all
my creatures were gone, and I did not know why." That is not a request for a
chronicle. It is a question about **causality, asked backwards, under time
pressure** — and a chronological list of events is the obvious answer to it
and very nearly the worst one.

It is also three failures, and only the third is a missing log:

1. **The question did not say who was asking.** `Pending::YesNo` carries the
   `AbilityRef`; `Prompt::YesNo` (`client-core/src/interaction.rs`) drops it,
   so the bar says "Pay the additional cost?" with no card. At a priority
   pass with a full stack the bar says "you have priority", not "passing lets
   *Wrath of God* resolve".
2. **The table did not explain the result.** Seven cards glide to the
   graveyard and then it is over. Nothing stays on screen.
3. **There was nothing to look up.**

So the design builds **one model** and puts it on **three surfaces**: into
the question, as a recap, and as a panel. A panel alone misses the problem,
because a player only opens it once they know they missed something — which
is exactly what they did not know.

## The model: episodes, not events

`Cause::Effect` says *that* an effect did it, not *which* — a journal entry
carries no source. Translating the journal line by line therefore answers
"why did my creatures die" with seven lines that never name the spell.

It does not have to. `resolve_stack_top` (`engine/progress.rs`) records
`StackObjectResolved` **before** the effects, so everything until the next
opener is that object's doing. An **episode** is that span, and the log is a
list of episodes:

```rust
pub struct LogEpisode {
    first_seq: u64, last_seq: u64,      // journal span
    turn: u32, step: Step,
    opener: LogOpener,                  // Resolved / Cast / Activated / Triggered /
                                        // LandPlayed / CombatDamage / StepBegan /
                                        // TurnBegan / Lost
    actor: Option<LogCard>,             // the card that did it, with its picture
    lines: Vec<LogLine>,                // already condensed: one per (kind, side)
}
```

Wrath of God is then **one** entry, **one** picture and **two** lines ("your
4", "their 3"), with the seven cards as thumbnails rather than as text —
grouped the way `CardGroup` groups the table, and for the same reason.

Three boundary rules that a test has to hold, because each is written the
wrong way by default:

- **A triggered ability is a property of the running episode, not of the
  event stream.** Inside a `Resolved` or `CombatDamage` episode it is a
  consequence (a dies-trigger during a Wrath); anywhere else it opens its
  own (an upkeep trigger). `AbilityTriggered` is also written for activated
  abilities, so the builder reads the `AbilityDef` at the source to tell
  them apart — an activation during priority is an action, not something
  that sank into whatever came before it.
- **The spell leaving the stack is the episode ending, not a death.** A
  zone change of the `actor` never becomes a line.
- **Sacrificing as a cost** (`ZoneChanged { cause: Cost }`, eleven sites) is
  a line of the `Cast`/`Activated` episode that paid it. That case *is* "my
  creature is gone and I do not know why", and it is what gives a `Cast`
  episode any lines at all.

Honest limit: two effects of one resolution ("destroy all creatures, then
draw two") cannot be told apart, and a replacement cannot be attributed.
Both want `Cause::Effect { source: ObjectId }` — about fifty call sites,
mostly in `resolve/`. The span rule carries every board wipe, every bolt,
every combat and every sacrifice without it.

## Where hidden information is cut

`baylee-gamehost/src/log.rs`, beside `view.rs` — not in the engine (the
journal is rules kernel and knows nothing of seats), not in the client (which
would then have the journal). One rule generalises every `view.rs` test:

> An event may name an object exactly when that object was public to this
> seat in the zone it **left** or the zone it **entered**.

Library → hand names nothing and becomes "draws 1 card"; hand → graveyard
names the card, it is face up now; a face-down creature dies as "a face-down
creature" for everyone but its controller. `Shuffled`, `ManaProduced`,
`ObjectTapped { cause: Cost }` are explicitly `=> None`. The exhaustive
`match GameEvent` with no `_` arm is the compile error a new event should
cause.

A `LogCard` carries a `CardIdentity`, so **`view.prints()` must walk the
log** — otherwise a seat holds an image key `GameStatic.prints` does not
have, which is precisely the hole the print table exists to prevent.

## The wire

`PlayerView.log`, `VIEW_VERSION` 13. `Session::snapshot` is `&self` and
cannot advance a watermark, so two routes into one type: a **pumped** view
carries every episode past `log_sent[seat]` (a connection property beside
`revealed`); a **snapshot** carries a window — the last 64 episodes, at most
two turns back. Two turns because "what did my opponent do on their turn" is
the question a reconnect has to answer; 64 so a combo turn does not blow it
open. Anyone away longer has lost the rest, and that is honest: a recap, not
a replay. The client deduplicates on `first_seq`, or a re-sent snapshot shows
an episode twice.

## The three surfaces

**The question.** `Prompt::YesNo` carries `source` through and the bar draws
the card as a chip. At priority with a non-empty stack: "passing lets *{0}*
resolve", and "… on *{1}*" when `StackItem.targets` names something of
yours. Targeted effects only — what an untargeted Wrath will do is not
knowable without card text, and a sentence that guesses is worse than none.

**The recap.** One line above the prompt bar, where a refusal already sits.
It appears unasked for an episode that took **two or more** of your own
permanents off the battlefield — two, because one creature dying is combat
and two at once is a board event. No life clause: combat damage is the most
routine episode in the game, and a recap that fires on every big attack is
one the player learns to dismiss, which rebuilds the original problem. Fires
only for `turn == view.turn`, so a cold start does not greet a player with a
massacre from two turns ago. Never modal; a tap opens the panel there.

**The panel.** Left, under the seat tabs, sharing its frame with the zone
browser (`hud/tray.rs`) but not its enum — the tray's invariant is that every
id it offers is drawn somewhere, and a log inside it would prove that for
nothing. `L` opens it, `Duel::log_open` defaults **false** (the lesson of the
black screen), and it never covers the prompt bar. The stack panel is on the
right and both are visible at once, deliberately: "what just happened" and
"what is about to" are the two questions of a priority decision.

An episode is drawn like a stack entry, because it is the same thing later:
the actor's picture, turn and step beneath it, the lines to the right. Hover
on a thumbnail sets `Duel::hovered` — the card lifts if it is still on the
battlefield, the pile chip lights if it is in a graveyard. `move_object`
returns the same `ObjectId`, so a logged card is still findable. No glow bit:
that register says what a player can *do*, and a log highlight is not an
offer.

## Language

Two exhaustive matches with no `_` arm: `GameEvent → Option<…>` in gamehost,
and `LogKind`/`LogOpener` → `Phrase` in `client-core/src/gamelog.rs`. About
**thirty phrases**, not hundreds, because no card name and no number is ever
*inside* a phrase — verbs are phrases, cards are arguments. What grows when
`GameEvent` grows is one arm in the gamehost match; a phrase only when the
event is a new *kind* of consequence.

## On a phone

`Metrics` is `pub(crate)` in `lobby/ui.rs` and knows only the lobby;
`docs/client.md` says the duel overlay has not had that pass. The log is the
first duel surface to ask. The numbers move to `client-core/src/metrics.rs`
(all `f32`); the one `Val` helper stays in the renderer, the way `keys.rs`
already splits it. A second, duel-only metric system would be the start of
two interfaces that disagree about the width of a phone.

## Not in the first cut

No full-text filter, no scrubbing, no replay, no jump marks. No lines for
mana, tapping as a cost, untapping, or phases as text. No end-of-game export.
No source on `Cause::Effect`. No team visibility — the log may not say more
than the view. No sentence in the bar that guesses at an untargeted effect.
No verbosity setting: first a version somebody has looked at.

## Order of work

1. `Prompt::YesNo` carries `source`; the bar draws the chip. No protocol
   change, and it is the most direct hit on the reported problem.
2. Types in `baylee-view`, `PlayerView.log`, `VIEW_VERSION` 13.
3. `gamehost/src/log.rs`, one test per sentence beside `view.rs`.
4. `Session`: watermark and window.
5. `client-core/src/gamelog.rs`: absorb, recap, phrases.
6. `Metrics` numbers into `client-core/src/metrics.rs`.
7. `hud/gamelog.rs` and the recap line; `L` in `keys.rs`.
8. `tests/duel_flow.rs`: a played-out duel's log boundaries agree with the
   `LocalHost` journal.
9. The priority sub-line last — its wording is decided after reading a log.

## The seat that stepped away

`SeatIdentity.away` exists (VIEW_VERSION 12) and nothing draws it — nor does
anything draw `is_ai`. `spawn_player_tab` reads name, team, life and counts;
a seat's **role** has no place in the tab at all. So this is not "how do we
draw away", it is "the tab needs a role line".

Both fields reach the client through `SeatPod` (`client-core/src/board.rs`),
which already carries everything the tabs and the mat say about a seat — so
the test "a chair the house is holding is marked away" is renderer-free.

What `away` is **not**: not `Standing` (that owns brightness and answers "who
are we waiting for" — the house may well hold priority, and then the mat
*must* be bright); not a colour change (the rim carries identity, and a seat
that goes grey says "gone", the one word this state must not say); not
`DEAD`.

What is left is **pattern**. The mat's rim goes from a solid band to a
**dashed** one in the same colour at the same brightness — a seam, not a
hole. The colour is there, the field is there, only the line is interrupted.
No pulsing in the first cut: motion would have to respect
`Preferences::reduce_motion` and would compete with the `ARMED` breathing.
The test beside the brightness ones: a held mat has the same mean rim
brightness as an unheld one, and has gaps where the other has none.

The tab gets a `MUTED` role line, **empty** for a present human — the quiet
is the statement — and otherwise "House · {profile}" or **"stepped away · the
house is playing"**. The two halves of that sentence are the two caveats:
*stepped away* is temporary, *the house is playing* means the chair is not
empty and the next question will be answered. Not "disconnected", not
"offline": those are words about sockets, and nobody at a table asks about
the socket.

The `▶` priority marker stays — if the house is thinking for the chair, the
table is waiting on that chair, and that is true. When the player returns the
line is empty again and nothing remembers, which is the point: a thirty-second
hiccup should not become a story.
