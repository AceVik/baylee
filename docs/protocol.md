# Protocol

Binary WebSocket protocol (protobuf, `baylee-protocol`, wasm-safe).
Schema: `crates/baylee-protocol/proto/baylee/v1/transport.proto`.

## Is it up? (`GET /health`)

Unauthenticated, and it has to be: a monitor that needs a token is a monitor
nobody wires up. There is nothing in the answer to protect — no token, no
account name, no store path, no configured URL — and the e2e suite asserts
that by *value*, not by field name, so a field added later cannot smuggle a
secret back in under another spelling.

```json
{
  "ok": true,
  "database": true,
  "catalog": { "state": "ready", "cards": true, "projection": true },
  "agents": { "connected": 1, "games": 2 },
  "games": { "running": 2, "waiting": 0, "seats_awaiting_engine": 0 },
  "version": "0.1.0+build.1057 (257eed7a28)",
  "commit": "257eed7a28df650934f50d4ff2557933d25ad713",
  "built_at": "2026-09-19T15:58:27Z",
  "dirty": false
}
```

**The status code carries exactly one question: the database.** `200` when
`ping` succeeds, `503` when it does not, and nothing else moves it.
`DATABASE_URL` is required, so a gateway whose database has gone away keeps
its port open and its log quiet while answering every route that matters with
a 503 — that is the one state this process cannot work around, and the one
this route exists for. Measured against a live gateway with Postgres paused:
`200`, `503`, `200` again on unpause.

Everything else is a field and never a code. **A gateway with no agent
connected is not unhealthy** — it hosts no games (`POST /lobby/games` answers
`503`, see *The gateway runs no rules*) and is otherwise correct, which is a
legitimate thing to be running and is what the e2e suite spawns three dozen
of. So `agents.connected: 0` is reported rather than escalated.

`catalog.state` is four values because they want four different things done
about them, and collapsing them to a bit hides the third:

| state | what it means | what to do |
| --- | --- | --- |
| `off` | the catalog's own schema could not be applied — most often a `unaccent` extension the role may not create | fix the grant; games still run |
| `empty` | reachable, nobody has ingested | `baylee-catalog ingest` |
| `projection_missing` | a full `cards` beside an empty `card_search` — **answers every search with nothing and errors at nobody** | `baylee-catalog project` |
| `ready` | both present | nothing |

Every probe is bounded at two seconds, because a health route that hangs is
worse than one that says "down": a monitor blocked on a socket reports
nothing, and nothing is indistinguishable from not-yet-scraped. The catalog
half is two `EXISTS` rather than `count(*)` — 0.61 ms against 7.47 ms on an
118 609-printing catalog, and only the count grows with the table.

`version`, `commit`, `built_at` and `dirty` are the same `baylee_build`
constants `GET /source` serves, and a test pins the two routes equal so a
second spelling cannot be introduced and drift.

**Wait on this route, not on the port.** An open port only says that `bind`
succeeded. It happens to be a sound readiness signal for this process — `main`
binds last, after the database and the catalog — but that is an accident of
ordering, and it stops being true the day somebody binds earlier to shorten
startup. Note also what it is *not*: a slow start is a slow start, and the
e2e harness waits 30 s rather than 5 s because five worktrees share one CPU
here and a gateway starting beside a compile needs longer than five seconds.

## Printings (which art the client draws)

Rules identity and *presentation* identity are two different things, and a
deck may legitimately hold the same card in several printings — three Islands
from three sets, one of them foil. So a card reference on the wire is a pair:

- `card_index` — the rules identity. The engine reads this and nothing else.
- `print_ref` — an index into a per-game print table. **The engine never
  interprets it**; it copies it onto the object at setup and carries it
  through every zone change, so the client is told exactly which of the
  duplicate cards it is looking at.

The table travels in `GameStatic.prints`: `scryfall_id`, `lang` and `finish`
per entry, which is everything the client needs to key the Scryfall CDN. It is
sent **per seat**, and an entry the seat has not earned is `None` — the table
is shared by the whole game and deduplicated per card, so a seat handed all of
it would be handed every decklist at the table. A seat knows its own deck's
printings from the start and earns the rest by seeing the cards; the host
re-sends the payload, ahead of the view that needs it, when one is earned. The
hole keeps the index, because the index *is* the `PrintRef`. The path end to
end is

```
DeckEntry { card, print }        (preset, per copy in the deck)
  → CardRef { index, print }     (engine, on the object)
    → CardIdentity { index, print, face }   (view, per seat)
      → GameStatic.prints[print]            (what to actually draw)
```

`baylee-gamehost`'s view tests follow one deck entry along that path,
including through a zone change, because a printing that silently reset to
entry 0 would be invisible in every other test — the game would play
perfectly and show the wrong art.

## Card text (`GET /catalog/text`)

Unauthenticated, like Scryfall's own answer to the same question, because a
client draws a readable card before it has an account.

```
GET /catalog/text?lang=de&oracle_ids=<uuid>,<uuid>,…   by card
GET /catalog/text?lang=de&ids=<uuid>,<uuid>,…          by printing
```

The first 500 ids are read and the rest are ignored. A malformed id is
dropped, not answered with an error, because one bad cast would cost the
whole batch its text. `lang`
defaults to `en`. Asked with neither list, the gateway answers `[]`. The
answer is a JSON array of `baylee_cardtext::CardTextEntry`, whose field
names are pinned in that crate. It is the one type both ends link:

```json
[{"scryfall_id": "…", "oracle_id": "…", "lang": "de", "layout": "normal",
  "faces": [{"name": "Gedankenstein", "english_name": "Mind Stone",
             "type_line": "Artifact", "mana_cost": "{2}",
             "oracle_text": "{T}: Erzeuge {C}.\n…",
             "printed": "{T}: Erzeuge {C}.\n…"}]}]
```

**Text is a property of the card, not of a printing.** Asked by card, the
gateway answers one entry per card it knows, and every face of it comes from
one printing, so a modal double-faced card is never drawn from two. Which
printing that is gets decided by `baylee_cardtext::pick` over all of the
card's printings in `lang`, the same rule a client applies to what Scryfall
tells it when there is no gateway:

- a printing that is really translated (not `NULL`, not the Oracle's own
  words);
- preferably one whose every face lines up with the Oracle;
- then the newest;
- then a tie-break that does not depend on row order.

`printed` is that printing's text exactly as Scryfall has it, the input
`baylee_cardtext::align` takes. **It is always the asked language and never
English.** It is `null`:

- under `en`, where the Oracle is drawn and a promo's own wording
  (`TAP: ADD G`) never is;
- on a face that printing left untranslated;
- on every face when no printing of the card translated anything.

In the last case the newest printing in the language still supplies the
name, and without one the newest English printing does, so `lang` is `en`.
`oracle_text` is what to draw for the whole face: `printed`, else the English
Oracle. The English Oracle a row is aligned against is not on the wire. The
client has it compiled in (`baylee_cards::oracle`), and it is the text the
line table was computed from. Aligning against a catalog's copy of it after
an errata would move rows.

A card the catalog lacks is not answered. The client falls back to Scryfall
and to the compiled Oracle.

**By printing** is kept for clients that predate `oracle_ids`. Each id is
answered with its card's entry under the id that was asked for, so an old
client gets the same choice. That is the one answer in which an entry names
two printings: `scryfall_id` is the one asked for, while `lang`, `layout`
and the faces are the served printing's. A printing the catalog lacks is fetched once
from Scryfall and kept, at most 25 a request, as before.

`oracle_id`, `layout` and `printed` were added with `oracle_ids`, with serde
defaults. A new client reads an old gateway's answer, and an old client,
which refuses no unknown field, reads a new one. An old gateway refuses
`oracle_ids` alone with a 400 (it requires `ids`), and a new client takes
that as no answer.

### How long an answer is held

A gateway holds the pool's text in memory, one copy per language: `/pool`'s
whole answer, serialized once, and each pool card's `/catalog/text` entry. A
card outside the pool is read from Postgres on every request. Only the
languages in the catalog's `languages` table are held. Any other `lang` is
answered in English, which is also what the catalog would have picked for
it, so a client cannot make a gateway hold a copy per code it made up.
`/pool`'s `lang` then says `en`.

The copy is keyed on the catalog's data stamp, `catalog_meta.data_version`.
`baylee-catalog ingest` moves it when it starts and again when it finishes,
never per batch. Every request reads the stamp, one primary-key lookup, and
a language held at a different stamp is read again: once, however many
requests arrive together. After the start stamp the gateway may hold a
catalog the ingest has half rewritten, where every card has valid text but
not yet its final pick. The end stamp replaces it.

Anything else that writes `cards` or `card_faces` goes unseen until the
stamp moves: a hand-run `UPDATE`, a restore. Move it by hand or restart
the gateway. By hand means the statement `Catalog::bump_data_version`
runs, because a catalog no ingest has stamped yet has no row to update:

```sql
INSERT INTO catalog_meta (key, value) VALUES ('data_version', '1')
ON CONFLICT (key) DO UPDATE
SET value = (catalog_meta.value::bigint + 1)::text;
```

A row added to `languages` needs the restart.

**Scaling out.** The one writer that does not move the stamp is the
gateway's own fill (`ids=` above), because a stamp would make every gateway
rebuild every language it holds. The filling gateway re-reads just the
cards it filled instead: their text in each language it holds, a query
each, and their names once. So a *second* gateway on the same database sees
another gateway's fill only at the next ingest. Until then it serves that
card's previous pick, which is valid text without the one new printing.
Lift this before running more than one gateway: give the fill a stamp of
its own, or move the fill out of the gateway.
`crates/baylee-gateway/src/texts.rs` has the mechanism.

## Card art (`GET /art/…`)

The gateway mirrors printing images on disk, and the route is a **mirror of
Scryfall's own path shape** so a client swaps one base URL and changes nothing
else:

```
GET /art/{size}/{face}/{a}/{b}/{scryfall_id}.jpg
        │      │      └──┴── the first two characters of the id
        │      └── front | back | backs
        └── small | normal | art_crop
```

`a` and `b` are redundant — they are derivable from the id — and are
**checked** rather than ignored, so one printing cannot end up cached under two
names. Anything else is a 404 before a request leaves for the origin.

`backs` is the one value in the middle that is not a face, and it is there
because the route has a segment there and a card **back** has no face. It is
the *shelf* Scryfall keeps backs on — a separate host, `backs.scryfall.io`,
addressed by size and id with nothing between them — so the mirror translates
`/art/normal/backs/0/a/{id}.jpg` into `https://backs.scryfall.io/normal/0/a/{id}.jpg`
and caches it beside the printings. Note that `back` and `backs` are different
things and both are real: `back` is the second side of a double-faced
*printing* and comes off the ordinary shelf. The client's half is
`baylee_client_core::images::back_url_at` and `BACKS_SEGMENT`; when it talks to
the CDN directly it uses Scryfall's own shape and skips the segment entirely.

This is the contract between `baylee_client_core::images::image_url`, which
builds these URLs, and `baylee-gateway`'s `art.rs`, which answers them. The two
crates cannot see each other — the gateway has no client dependency, and it
must not gain one — so **each side tests the shape against this section**
rather than against a shared constant.

Three properties are load-bearing:

- **It is a cache keyed by an id, never a proxy.** The only thing a caller may
  name is a printing id; the origin URL is rebuilt from the fixed shape above.
  There is nothing to point at another host with, which is what makes an
  unauthenticated route safe to leave open — and it is unauthenticated on
  purpose, because it serves public artwork the client would otherwise fetch
  straight from a public CDN, and a token would break plain image loads while
  protecting nothing.
- **One rate limiter for the whole gateway**, not one per game: `docs/legal.md`
  §3 allows ten requests a second, and four tables starting at once must not
  leave at four times that.
- **`Access-Control-Allow-Origin: *` and a one-year `Cache-Control`.** The
  browser client is served from one origin and talks to the gateway on another,
  so without the first it loads no art at all and fails silently. The second is
  the client-side half of the cache, and on wasm it is the only half there is —
  a printing's art never changes, because the id *is* the version.

### Warming a table

When a game is created the gateway walks the preset's whole print table and
fetches it in the background. This is the one thing only the gateway may do: it
builds the `GamePreset`, so it legitimately knows every deck at the table,
while a *seat* knows its own printings and earns the rest by seeing the cards
(see "Printings" above). Warming in a client would therefore hand a player
their opponent's decklist. Warming here hands nobody anything — a client still
learns a printing only when the rules let it, and only then asks for the
picture, which is by then already local.

`BAYLEE_ART_PATH` names the directory and `off` disables the mirror entirely,
which is a real mode and not a degraded one: a gateway behind a CDN has no use
for a second copy, and the test suite must never reach the network. With the
mirror off, `/art` answers 404 and a client falls back to fetching from
Scryfall itself.

## Standing answers

A seat can tell the engine "always say yes to this ability" — that is the
`answer` of `PlayerAction::SetAbilityPolicy { ability, pass, answer }`,
addressed by `AbilityRef { card, index }`. The handle names a *card's* ability
and nothing about the table it is set at, which is what makes it a preference
an account can keep: the client holds its policies in the preferences
document (`ability_orders`, [`/settings`](#client-preferences-settings)) and
sends them as seat actions once it has a view, and again whenever one
changes. Setting one is not a game action (the pending question stays
exactly as it was), so a reconnect simply restates what the seat already has.

The gateway neither stores nor replays them. It used to (`/automation`, and
`SeatAttached.standing_json` replayed into the seat before the first pump);
no client ever called the route, and #233 retired both on 2026-09-24.

A stored answer covers one **kind** of question, and not every question a
card asks. The engine's gate is `YesNoPrompt::automatable`, and it is true
for two variants: `MayDo`, the printed "you may" inside a resolving ability,
and `CommanderZone`, which is what `AbilityRef::COMMANDER_ZONE` below was
reserved for. Everything else a card asks is a decision about *cost* — a
kicker, a shockland's two life, a tax trigger, a miracle — and those carry an
`AbilityRef` too, so a rule keyed on "the question names an ability" would
have let the first standing answer a player ever stored spend their mana.
That was the rule, until `MayDo` gave it something to be wrong about: a seat
that had said "always take Ondu Cleric's life" would have had Rite of
Replication kicked for it. `CommanderReplace` is the near miss that stays
off the list — its right answer depends on the `to_library` the prompt
carries, and a stored bool cannot see it. A new variant is automatable only
when saying yes to it costs nothing but the choice.

Nothing checks a kept handle against the registry: `/settings` is opaque to
the gateway, and the engine files whatever handle it is sent. One that names
no ability this build has never fires, and fails silently — the seat is simply
asked a question it believed it had answered for good.

The reserved indices are therefore **wire constants**: a stored answer is a
number, and moving one silently re-points every account that holds it. They
live in `baylee_core::ids::AbilityRef`, count down from `u32::MAX`, and a test
there keeps them a set with `FIRST_RESERVED` at its floor. The newest is
`COMMANDER_ZONE`, CR 903.9a's "put your commander into the command zone?" —
the odd one, in that the rules ask it about a card that prints nothing of the
sort, and an ordinary one in that a player may want it answered once and for
good.

What is *not* in that space, despite counting down from the same `u32::MAX`,
is `baylee-engine`'s `choice::granted_ability(n)` and `choice::PREPARED_CAST`.
Those name a slot in one `LegalActions`, chosen fresh every time it is built
and held by nothing outside that game, which is why an ability the engine puts
on the stack carries `AbilityRef::SYNTHETIC` rather than the slot it was
offered under.

The other half of that handle **is** honest, and was not. `AbilityLoc.card`,
which a `StackItem::Ability` carries out as its `AbilityRef`, is an
`Option<CardIndex>` (view version 16), because a card-less source has no card
to name — an emblem (CR 114.2), a token, and since token copies were handed
the rules text they copy (CR 707.2) a copy of anything. All three used to be
given `CardIndex::new(0)`, which is not a free sentinel: index 0 is a real
card in the ledger, so a client looking that handle up labelled the ability
with a stranger's text, and a standing answer filed under it covered every
such ability in the game at once. The picture was always right, because
`StackKind::Ability { source }` names the permanent rather than the card, so
what it cost was a line of text and an "always say yes" that said more than
it meant.

The same `Option` mattered inside the engine, in a way the handle did not.
Six places read the card out on their way to writing one down, and two of
them treated *not having one* as a reason to stop: the two branches that put
a synthetic keyword trigger on the stack returned early, so a source with no
card was queued a trigger and never fired one. A copy of a warded or
prowessed creature is exactly that shape. They fall through now — the card
is identity, and identity is not a precondition — and a loyalty activation
and a copied spell's scry rider stopped being refused for the same reason.

## Priority holds (view version 9)

The other half of a standing answer is `PlayerAction::SetPriorityHold`, and
it is deliberately **not** kept per account: a hold names a condition inside
one game ("until this stack empties", "for the rest of this turn"), so there is
nothing about it to carry to the next table. It is a game action in the same
sense a standing answer is — it changes no pending question — and like one it
is accepted from **any seated player at any time**, not only from the seat
being asked. `Engine::apply` handles `action.is_automation_setting()` before
the "who is being asked" gate, and `Session::act` checks only that the seat is
human. Without both of those a hold could be set and never taken back, because
a held seat is by definition not the one being asked.

Every `PriorityHold` cancels itself. `UntilStackEmpty { depth }` ends when the
stack empties **or** when anything is added above `depth`, which is what makes
a stale client safe: a view is a snapshot, so a client that sends the depth it
last saw is sending a number the engine may already have passed — and the
engine reading a larger depth cancels the hold, which is exactly the right
answer, because somebody just responded to what was being let through.

What the client is told is one bool, `PlayerView::priority_held` — **the
viewing seat's own hold and no other seat's**, because a hold is a statement
about what its owner intends to respond to and telling the table would hand
out precisely the read a player is entitled to keep. A bool rather than the
enum for two reasons: `baylee-view` does not depend on the rules kernel, and
the client has only two questions (light the indicator; does the key set or
cancel), neither of which the flavour changes. `PassWhenNothingToDo` reports
as **not** held — it answers only where passing was the seat's sole legal
action, so it never withholds a decision; `PriorityHold::suppresses` is the
one place that partition is written, and `auto_answer` reads the same function
so an indicator cannot disagree with the engine. That is
**`VIEW_VERSION` 8 → 9**.

`crates/baylee-gamehost/src/view.rs`'s `player_view` therefore takes the hold
as a parameter, the way it already takes `priority`: both live in the engine
rather than in the `GameState` the view is built from, so only the caller can
read them.

## Granted mana (view version 11)

A land under a Chromatic Lantern taps for any colour, and there is no card
anywhere a client can read that off: the ability exists only in the engine's
effect table, offered under the synthetic index `choice::GRANTED_ABILITY`.
The client therefore knew the *handle* and not what came out of it — its mana
planner counted such a land for nothing, and the player tapped those lands by
hand while everything else was planned for them.

`PublicObject::granted_mana` is a projected characteristic in exactly the sense
`power` and `subtypes` are, and is carried for the same reason: a client cannot
run the layer system. It is `Option<GrantedMana { colors, amount }>` — "n mana,
of one of these colours" — and `None` both for a permanent with no such ability
and for a grant too complicated to reduce to that sentence, which keeps the
honest-stub rule the card pool already obeys. It also names **which** granted
ability it describes (`slot`), because a permanent may be granted several and
the mana one is not necessarily the first: Urza's Saga is granted chapter I's
`{T}: Add {C}` and chapter II's `{2}, {T}: Create a Construct`, and a client
told only "this makes mana" would tap the slot next to the one it was
promised. A plain ordinal, not the engine's synthetic index — `baylee-view`
does not depend on the rules kernel and `choice::granted_ability` is the
kernel's encoding. That is **`VIEW_VERSION` 9 → 11**, two bumps in one night:
10 added the field and 11 added the slot to it.

Two functions rather than a third copy of the rule. `effects::granted_activated`
is the engine's own lookup: `legal_actions` offers the ability through it,
`start_granted` runs it, and `crates/baylee-gamehost/src/view.rs` projects it.
`baylee_cards_dsl::simple_mana` is the reading — free cost, a single `AddMana`,
a fixed amount, no restriction — and the client's `manasources` asks it of a
*printed* mana ability. An offer and a projection that disagreed would be a
land the planner counts on and the engine refuses, with the rest of the plan's
lands already tapped, so they are one function each and the gamehost test
asserts the two answers against each other. Both walks stop at
`choice::GRANTED_SLOTS`, so they agree at the bound as well as below it — a
ninth grant projected as slot 8 would come back as `PREPARED_CAST`, an index
in the same space that means something else entirely.

## What a seat owes (view version 24)

A player who agrees to pay ward's tax is handed a mana window (CR 605.3a) so
they can make the mana. The window is deliberately shaped like nothing: an
ordinary `Pending::Priority` offering mana abilities and no new question
type, which is what lets a client draw it and an agent answer it with what
they already have. That is also why neither could tell it apart from a quiet
priority pass with no plays — the house agent said yes, saw two untapped
Plains and nothing castable, passed, and its own spell was countered.

`PlayerView::owed` is `Option<ManaCost>`, the **total** that was asked, and
is read with `PlayerView::awaiting`, which already names who owes it: the
payer holds priority inside its own window, so the two fields are one
sentence. It must not be inferred — deducing "I owe something" from an offer
of mana abilities with nothing castable would tap lands in every other quiet
window too.

A cost and not a number, on two measurements. The engine's `u16` is generic
because `Effect::PlayerMayPayOr` carries an `Amount`, a tax being allowed to
be its own source's power — a statement about *when* the number is known, not
about what it may hold — and by the time a window is open it has been
evaluated, so the view inherits none of that. And both readers on the far
side already take a cost: `manapip::cost` draws one, `manaplan::plan` solves
one, so a number would be converted at each of them on the way in.

Mana-only by construction rather than by omission. The other payment this
engine can ask for — a Karoo's "return an untapped Plains you control",
`AwaitingOp::PlayerMayPayCost` — is answered by naming an object from a list
the pending choice already carries, and opens no window at all.

One projection, `view::owed_payment`, for the same reason `granted_activated`
is one lookup: an offer and a projection that disagreed would be a window the
player is told to use and a price the engine does not charge. It is gated on
the window and not on the suspended resolution, which holds the price for the
whole of the yes-or-no question too — reading the operation alone reports a
debt while the seat is still deciding whether to take one.

## How long a seat has left (view version 25)

The decision clock has run since the engine moved into a process of its own,
and until now it reached no seat at all. On the default preset that means a
player saw nothing for ten minutes and then lost a decision in silence.
`PlayerView::decision_remaining_ms` is the number, and three decisions about
it are worth more than the field is.

**Relative milliseconds, not a deadline.** An absolute instant would make the
client's own clock a rules question: a seat whose machine runs a minute fast
would draw a minute it does not have, or lose one it does. A client counts
down from the number and takes the next view as the correction, so the only
thing that has to be right is the host's own arithmetic.

**Public.** Every seat is told the awaited seat's remainder, not only the
seat on the clock. A table where one player is running out of time and nobody
else can see it is a table where the pause reads as rudeness rather than as a
clock. It is read with `PlayerView::awaiting`, which names whose remainder it
is, exactly as `owed` is above.

**`None` is four situations wearing one answer**: nobody is being asked, the
table is `untimed` (`decision_timeout_secs` at zero — no number because there
is no limit), the awaited seat is an AI chair, or the awaited seat is on the
**stand-in** clock rather than the decision clock. That last one is the one
worth stating: its socket is gone, so it is not deciding at all, and a
countdown drawn against it on everybody else's screen would name the wrong
thing happening. Zero would be a seat with no time left, which is why the
field is an `Option` and not a sentinel.

### Why the number is handed in rather than read

`baylee-gamehost` may not read a wall clock. The rules kernel is
deterministic and a session that timed itself would replay differently on
every machine, so the one value here made of elapsed time arrives from
outside: `EngineRunner` resolves it and `Session::set_decision_remaining`
stores it **with the `decision_seq` it was true of**.

That anchor is the whole of `Session::decision_remaining_ms`. A reading only
describes the question it was taken for, so once the question has moved the
reading is discarded and the seat is given the table's whole allowance — a
question just asked has had no time taken off it. Without that branch the
first view of every new question would carry the previous question's
leftovers, and a seat would be shown four seconds to answer something it was
asked a moment ago. It is anchored to `decision_seq` and not to a timestamp
for the reason that counter exists at all: it counts *questions asked* rather
than frames sent, so an opponent's priority hold cannot wind it.

The resolution happens twice per frame, and the second time is not
redundant. Registering a socket moves the awaited seat off the reconnect
window and onto the decision clock, and that happens after the frame has
arrived and before any view is built — including the snapshot a resyncing
player is sent, which is the one view in the system whose whole job is to
tell a returning seat where it stands.

### A clock-shaped field must not reach a rules decision

The host builds this number into the views it sends to sockets and leaves it
`None` in the views it hands its own agents. `HeuristicAgent::act` takes a
`PlayerView`, so an agent that read a remainder would answer the same
position differently on a slow machine than on a fast one — legally, and
invisibly, because nothing about the resulting play looks wrong. Machine
speed is not an authorized input to a decision; the invariant is that with
identical authorized inputs an agent's answer cannot change, and it is the
same invariant as the house AI drawing from the game's own seed.

Anyone adding another field of this shape to `PlayerView` inherits the rule:
if it is made of wall time, it belongs in the views that go to sockets and
nowhere else.

### The two limits, stated at join (view version 26)

The remainder above is what moves; `GameStatic::decision_secs` and
`GameStatic::reconnect_secs` are what it starts from. Since a room picks its
own clock they are per table, and until version 26 a client was told neither.

**Join is the only moment for the reconnect window.** A client is
disconnected for exactly the window it would be counting, so no in-game
payload can reach it while the number matters. That alone puts it on
`GameStatic` rather than on a view.

**And the lobby row is not enough for either.** `GET /lobby/games` has
carried `clock: { decide_secs, reconnect_secs }` since the room learned to
pick a clock, but three ways into a game read no lobby row at all:
`xtask dev-table`, a rematch, and taking over an AI chair. A seat sheet fed
from the listing would be right for a lobby game and blank for the rest,
which is the kind of half-working that is noticed by the first player to
take a rematch.

So they ride the payload every seat gets on every socket, beside the roster
and the print table — the same shelf as everything else that is known at
join and constant for the game.

**Both are `Option<u32>` because the house rules spell *no limit* as zero,**
and zero on a screen reads as the opposite: a seat with no time at all. A
gateway room cannot choose either zero for the window
(`clock::MIN_RECONNECT_SECS` is 10) but a local harness can, and the payload
has to be honest about a table this gateway did not make. One reading,
`view::no_limit_is_none`, does the translation, so no client has to know that
the rules and the wire disagree about how to say "never".

These are the *room's* numbers and `PlayerView::decision_remaining_ms` is the
*question's*; they are deliberately not one field. A fraction of the limit
would show a thirty-second table its clock for three seconds, which is why
the limit is stated once here and the warning threshold is flat.

## Why a seat lost, and who answered for it (#83)

`SeatView::loss` is why a seat is out of the game, and `None` while it plays
on: `Life`, `EmptyDraw`, `Poison`, `CommanderDamage`, `Conceded`, or `Effect`
when an effect said the seat loses, such as a pact left unpaid (CR 104.3e).
The type is `LossCause`, a mirror of the engine's `LossReason`, so an engine
rename cannot move the wire. It replaced the bare `has_lost` flag, which is now
the method `SeatView::has_lost()`. It is public, because a seat going out is
announced at a real table and so is why. The engine keeps the first loss:
a seat that is already out and then concedes still lost the way it lost.

`SeatView::house_answered` is who answered the seat's most recent decision
in its place: `Clock` when its decision clock ran out with the socket
attached, `StandIn` when the socket was gone and the house was holding the
chair. `None` means the seat answered its last decision itself.

- **It clears only on the seat's own next answer over a socket.** Reconnecting
  does not clear it (the last decision is still the house's until the player
  makes one), and neither does an automation setting, which is not an answer.
- **An AI chair never carries it**, driven or not. The roster's `is_ai`
  already says the house plays that chair.
- **It freezes at a loss**, because a seat that is out is asked nothing more.
  That is what lets a client draw a loss to the clock as one:
  `loss.is_some() && house_answered == Some(HouseAnswer::Clock)`.

**The host records it; the engine does not know.** The engine has one door,
`apply(player, action)`, and takes an action without asking who produced it:
a rules kernel with a second door would have a second set of rules. Only the
session knows which answers were its own, so it records them.
`Session::answer_by_clock` is the clock's door: `EngineRunner` calls it when a
`Deadline::Decide` expires, and it refuses (answers nothing) when the seat it
names is no longer the one being asked, so a timer that fired after its
question moved on cannot take another seat's decision. A stand-in is marked
where `Session::pump` plays it. Both marks are set before the views of that
answer are built, so the frames that carry the answer also say who gave it.

Two things this does not do. **A replay does not carry it**: a replay
re-applies actions, and who produced an action is not in the journal. That
comes with the game log (`docs/game-log-design.md`). **Nothing escalates**: the
clock answers one question at a time, as often as the seat keeps timing out,
and a player who never answers is answered for until the game ends. Whether a
run of clock answers should become a stand-in or a concession is a table
policy nobody has decided.

`EndReason` is unchanged. It says how the *game* was decided (CR 104), which
is the same sentence for every seat; why one seat lost is that seat's.

## Commanders (view version 13)

`SeatView` used to carry `commander_casts: Vec<u32>`, and it was two things
at once: built as `GameState::commander_casts` — one running total per
*seat*, which is what Commander's Insight counts — and read by the client as
the per-commander tax, indexed by the card's slot in the command zone. At a
duel where both seats have one commander the two shapes coincide exactly, so
the mistake was invisible: every seat but the first drew seat 0's number, and
a seat with partners could not have drawn the second commander's at all.

It is now `SeatView::commanders`, a `CommanderView` per commander — `object`,
`card`, `name`, and the `casts` that CR 903.8 charges `{2}` apiece for, taken
from `Commander::casts` where the tax actually lives. A client matches it to a
card by `object` and never by position. Beside it, `commander_damage` is the
second life total CR 903.10a defines: `CommanderDamage { source, amount }` for
each commander that has hit this seat, public to the whole table because
twenty-one is a number everyone at it is counting.

`PublicObject::commander` and `HandObject::commander` mark the card itself.
Strictly redundant — the ids are in `SeatView::commanders` — and carried
anyway, because every renderer that draws a card already holds one of these
and would otherwise need the seat list threaded down beside it; the gamehost
test asserts the two never disagree. It is also in `ObjectSummaryKey`, so a
commander never collapses into a stack with an ordinary copy of itself.

Unlike `PublicObject::card` beside it, the marker is **not** gated on what the
seat is entitled to see, and neither is `CommanderView::card`: CR 903.3
designates a commander openly, so every seat learned this identity from the
command zone before the first turn and is told it again in every zone —
including its owner's hand, where declining CR 903.9b leaves it. Gating one of
the two and not the other would have been worse than either answer, because
they are one claim.

Manifest is the case that will break this, and gating the marker would not
have fixed it. A commander manifested off a library is a card nobody
announced, and `CommanderView::object` names its handle: blanking the card
field still leaves it identifiable by cross-reference against the face-down
permanent. That needs the handle hidden too, and nothing sets `FACE_DOWN` yet.

`PlayerView::prints` walks the commander lines, which it has to. That hand is
a zone no other seat's view walks, so without it a seat is told a card
identity whose printing it was never given, and draws a hole where the card
should be.

## What an ability on the stack says it does (view version 17)

`StackItem::Ability` carries a third field, `text: Option<StackText>`, and
what it names is a **sentence of a printing**: `{ face, line, of }` — which
face of the card the ability was printed on, which of that face's sentences
it is, and how many sentences that face has. The client owns the text in the
player's own language, so the host sends a coordinate rather than prose and
nothing on this wire has to be translated.

It is a field beside `ability` rather than more of `AbilityRef`, and that is
the whole design. The handle above is what a **standing answer** is filed
under: it names an ability across games and across printings, and it is a
wire constant for exactly that reason. A sentence index is about one
printing's text — a different printing of the same card can lay its lines out
differently, and a reprint is free to. Putting one inside the other would
make every stored answer depend on which piece of cardboard was in front of
the player when they gave it.

`of` is the guard, and it is why the field is a triple rather than a pair.
An index out of range is caught by anyone; the case that needs catching is a
text one sentence *shorter*, where the index lands in range and points at the
sentence beside the right one — precise text that is confidently wrong, which
is worse than the label it replaced. So the count travels with the index and a
text of a different length is refused whole. The count is of the **English**
text, because that is what the host generated its table from
(`baylee_cards::lines`), and a translation that splits its lines differently
is a translation the client will not index into.

The face is the one that is easy to get wrong, and the trap is that the wrong
answer is right most of the time. An ability on the stack is independent of
its source (CR 113.7a): the source may have transformed, or left the
battlefield entirely, while the ability waits. Reading the source's *current*
face therefore names a sentence the ability never came from — and on a card
whose two faces have the same number of sentences, `of` cannot catch it,
because both counts are English. The host answers from the ability list the
object took with it when it went on the stack (CR 608.2), which
`crates/baylee-gamehost/src/view.rs` recovers the face from by identity.

The field is absent — and the client falls back to the label it drew before —
for an ability whose source is a token, a token copy or an emblem (there is no
printed card to index into, CR 111.1 and CR 114.2), for one a continuous
effect granted, and for a keyword ability that is printed as a word rather
than as a sentence of its own.

## Client preferences (`/settings`)

Keys and standing orders follow the **account**, not the machine: a player who
rebinds confirm at home finds it rebound at a friend's table.

- `GET /settings` → the stored object, or `{}` for an account that has never
  saved any. Never a 404: the client's own defaults are the right answer, and
  making it tell two failures apart buys nothing.
- `PUT /settings` replaces it. The body *is* the preferences object — there is
  no wrapper, because there is nothing else to say about it.

The gateway keeps the blob **opaque** and checks exactly two things: that it is
a JSON object, and that it is under 16 KiB. It cannot check more, and should
not: knowing what a keymap is would mean linking `baylee-client-core`, which
is the client's brain and pulls in the engine behind it — the one dependency
the gateway does not have. The second reason is deployment order: a client
that learns to remember a new preference must not need a gateway release
before it can store it.

The shape is `baylee_client_core::prefs::Preferences` — a `Keymap`, the phase
rail's `PhaseOrders`, and the `AutoRules` switches — and every field of it is
`#[serde(default)]`, so a blob written by an older or newer client still loads
with the rest defaulted rather than costing a player their bindings. A
corrupt blob decodes to the defaults rather than to an error, for the same
reason: preferences are a convenience, and a player mid-upgrade should get a
working keymap rather than a screen that will not open.

Not stored here: the preview's size, the interface language, and the gateway
address. Those are properties of a *device*, they stay in the client's own
local store, and putting them in the account would mean a phone and a desktop
fighting over one number.

## v0 (M0)
Transport handshake + preset transfer:
`Hello{protocol_version, card_pool_hash}` / `HelloAck`, `JoinGame`,
`ResumeGame{last_seq}`, `GamePresetMsg`, `Heartbeat`, `Error`, wrapped in
an `Envelope` oneof. Card references are `{card_index, print_ref}`;
`card_pool_hash` invalidates client caches.

## v1 (M3, shipped 2026-08-29)
`CreateGame` / `GameCreated`, `ChoiceRequest{game_id, seq,
pending_json}`, `PlayerActionMsg{game_id, seat_token, action_json}`,
`StateDelta` (reserved). **Design decision (documented):** the full
engine choice taxonomy (`Pending`, `PlayerAction`) travels as
**serde_json payloads inside the protobuf frames** — the wire stays
binary protobuf, but the taxonomy evolves without proto churn. A typed
protobuf mapping of the taxonomy is a protocol v2 item, together with
per-player hidden-information filtering (`FullView`/`Delta`), timers
(`TimeExtensionRequest/Vote/Result`), `SetAutomationRules`, dev-mode
`DevCommand`, and spectator streams.

That seam has since paid for itself three times, and every time the feature
was filed under v2 before anyone checked what it actually touched: the copy
target re-choice (CR 707.10c), the agreed draw (CR 104.4i), and attacking
planeswalkers all shipped as `Pending`/`PlayerAction` changes with **no
proto change at all**. Before scheduling something behind protocol v2,
check whether it needs the wire or only the taxonomy the wire carries.

## Capabilities, not a dev flag

`GamePreset` carried `dev_mode: bool` — "enables `DevCommand`s (never in
normal lobbies)". Nothing ever read it, and it arrived **inbound** in
`GamePresetMsg`, so the one thing it could do was let whoever opened the
socket ask to be trusted. Field 3 is now `reserved`.

`SeatSpec::capabilities` replaces it: per seat, `Default` is nothing, and
the host is the only thing that grants any. `dev_commands` is what the
engine checks — `Engine::dev_state_mut(seat)` returns `None` for every seat
without it, where the old `state_mut_dev()` took no seat and asked nobody.
`see_hidden` is reserved for a judge or replay view and is granted by
nothing yet.

The gateway builds lobby presets with no capabilities at all, and
`gamehost::preset` has the test that says a wire preset cannot grant itself
one.

## AI views and private scouting

`HeuristicAgent::act(&PlayerView, &Pending)` uses the same filtered view as a
networked player. The host can additionally provide borrowed selected-effect
context and a private scouting report to an active house AI. This is an
intentional AI privilege: current hands, submitted decks, sideboards and
requested library depth. None is added to `Pending`, `PlayerView`,
`GameStatic`, or a protocol envelope, so this does not change the view version.

The authorization lives in `gamehost::scouting::request`, which checks the
current controller for every call. Only `SeatKind::Ai` qualifies. A `Driven`
AI chair is being played by a human and is refused; `StandIn` belongs to a
human and is also refused. No network request can invoke this private module.
Reports have no serialization and are consumed for one decision without
teaching the print-disclosure table or retaining hidden cards in an agent.

Two fields moved into the view to make that possible, and both are things
a human client wanted anyway: `SeatView::mana_pool` (floating mana is
public at a real table, and the seat deciding whether to tap another land
needs it) and `PublicObject::mana_value` (what a spell on the stack
actually cost). That is **`VIEW_VERSION` 5 → 6**.

`play_game` moved from `baylee-ai` to `baylee-gamehost::harness` for the
same reason: building a view takes the engine, which is the boundary the
agent may not cross.

The agent does depend on `baylee-cards`, and that is not a hole in the
same wall. `LegalActions` names an activated ability as a
`(source, index)` handle and carries nothing about what it costs or does,
so deciding whether to use one means reading the registry — which is
public card data, identical for every seat, and exactly the lookup the
client makes to label the same buttons. The line is *state* versus
*printed text*: a registry answers "what does Arid Mesa say", never "what
is in that library".

## Combat enumerates its own answers

`Pending::ChooseAttackers` carries `attackers` beside `defenders`, and
`Pending::ChooseBlockers` carries a `BlockOption` per creature that may
block, naming the attackers it may block. Evasion is a pairing question,
so a flat list of "creatures that may block" would be wrong for every
flier on the table. `CombatCandidates` — the client's own guess at both —
is gone. No proto change: the taxonomy travels as JSON.

## The gateway runs no rules

A game does not live in the gateway. It lives in an engine process that an
**agent** started and that dialled the gateway back; the gateway routes between
that process and the seats, and links neither `baylee-engine` nor
`baylee-gamehost`. The circle:

```
POST /lobby/games ─┐
                   v
              gateway ── StartEngine{game_id, engine_token, gateway_url} ──> agent
                   ^                                                          │ spawn
                   │                                                          v
                   └──────── EngineHello{game_id, token} ──── baylee-engine-server
                   │
                   ├── GameSetup / SeatAttached / SeatDetached / SeatFrame ──>
                   <── SeatFrame / GameEnded ──────────────────────────────────
                   │
                   └── the seat sockets, unchanged
```

Three sockets, three secrets, and no two of them are interchangeable:

| socket | who opens it | what proves it |
| --- | --- | --- |
| `GET /agent/ws` | an agent | `BAYLEE_AGENT_TOKEN`, from the gateway's own configuration |
| `GET /engine/ws` | one engine process | a token issued for exactly one game |
| `GET /games/{id}/ws` | a player | a seat token, scoped to one seat of one game |

A player's token opens neither of the first two. The engine token is minted
when the game is ordered, handed to the agent, and passed to the process it
starts as a command-line argument — it never leaves the machine the agent runs
on, and it is worth one game.

**`SeatFrame{seat, envelope}`** is what makes the routing possible without the
gateway understanding a word of the game: it nests an *encoded* player-facing
`Envelope` and tags it with a seat, so the gateway forwards the bytes it was
handed without decoding or re-encoding them. The player-facing protocol keeps
exactly the shape it had; a client cannot tell it is being proxied.

Two things moved out of the gateway with the rules:

- **The decision clock.** It has to sit where `awaiting_seat()` and the
  session's counters can be read, which is now the engine process. It is
  anchored to `Session::decision_seq` — how many *questions* the game has
  asked, not how many frames it has sent — so one seat's expired clock can
  never take another seat's decision, and it does not run for a seat with no
  socket, because a player who walked away is not on a clock they cannot see.
  The distinction between the two counters is not cosmetic: a priority hold and
  an ability's policy are the only things the engine takes from a seat that is
  *not* being asked, and a client restates its policies whenever it
  reconnects, so a clock anchored to `seq` would restart every time the
  opponent pressed `F6` or reconnected. That is unlimited thinking time for
  whoever spams either.
- **The panic boundary.** One process per game *is* the boundary, so the
  `catch_unwind` the gateway used to wrap every rules call around is gone. A
  rules path that panics takes down exactly one game, and the agent reports the
  exit.

A seat's frames are dropped in the *engine* while it has no socket, not one hop
later at the gateway. That is what keeps a seat's own opening payload first on
its wire: the frames another seat's arrival produced for a player who was not
there yet are gone before they can overtake it. Nothing is lost by it — every
attach pumps, and a pump re-sends the current view to every seat that is
present.

Losing the engine link ends the game. The state lives in that process and
nowhere else, so a link that closes before `GameEnded` is a game that cannot be
continued; the gateway marks it over rather than leaving a table that will
never move again.

### Running one

```bash
BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32) ./target/debug/baylee-gateway
BAYLEE_AGENT_TOKEN=<the same> ./target/debug/baylee-agent
```

The agent finds `baylee-engine-server` beside itself (`BAYLEE_ENGINE_BIN`
overrides), reconnects with backoff, and takes `BAYLEE_GATEWAY`,
`BAYLEE_AGENT_NAME` and `BAYLEE_AGENT_CAPACITY` (0 = no limit). The gateway
tells an engine to dial `BAYLEE_ENGINE_URL`, which defaults to
`ws://127.0.0.1:{PORT}/engine/ws` — right for a single box, wrong the moment an
agent runs somewhere else. With no agent connected, `POST /lobby/games` answers
`503`: there is nothing to run the game, and handing out a seat token for a
table that will never start would be worse.

## Confirming an address, and why it is optional

`BAYLEE_SMTP_URL` decides the whole feature. Without it the gateway has no
mailer, `POST /auth/register` marks the account confirmed on creation, and
everything behaves exactly as it did before confirmation existed — which is
the development default and what every other test in the suite assumes. With
it, a fresh account gets a link by mail and `POST /auth/login` answers `403`
`confirm your e-mail address first` until the link is followed.

Three details are load-bearing:

- **The confirmation check runs after the password check.** Answering "confirm
  your e-mail" to a *wrong* password would tell a stranger that the address
  exists, which is the one thing every other answer on that route is careful
  not to say. `POST /auth/confirm/resend` answers `{"ok":true}` for the same
  reason, whether or not there was anything to send.
- **Only the hash of the link's token is stored**, like a session token's:
  the store is a file on disk, and a live link in it would be a live login.
  A link lasts 24 hours, a new one invalidates the last, and following one
  spends it.
- **`BAYLEE_PUBLIC_URL` is where the link points.** The gateway cannot work
  out its own public address, and taking it from a request header is how a
  confirmation link ends up pointing at whatever `Host:` an attacker sent.

`GET /auth/config` reports `confirmation_required` beside
`registration_enabled`, so a client can say "check your e-mail" instead of
trying a log-in that is going to be refused. The mail itself is written in
the `lang` the account registered with — kept on the account, so a resend
months later still lands in the language the player signed up in.

**A new key here is safe in one direction only.** A client's config struct
deserializes the keys it names and ignores the rest, so a key it has never
heard of — `clocks`, below — reaches an old client as nothing at all. That is
a property of that struct and not a promise this route makes: a field a
client is *required* to read breaks every client that predates it while the
same sentence stays true, and is a version bump rather than a new key.

## A name is not a claim: `Alice#af03`

A display name is **not** unique. Two players may both register as Alice, and
what tells them apart is `account.tag` — an identity column, so the database
hands it out and nothing in the gateway picks one.

A tag is drawn as **lowercase hex, padded to four digits and allowed to grow
past them**: `#0001`, `#af03`, `#10000` for the account after the sixty-five
thousand five hundred and thirty-sixth. That last case is the whole reason
the width is not fixed. Discord's four digits worked because the pair
`(name, discriminator)` was the identity — two people could both be `#0001`
under different names — and a tag that is unique on its own is a user number
instead, which runs out.

A handle is `name` + `#` + that, which is why
`auth::valid_display_name` refuses `#` inside a name, along with `@` (a name
on a screen that also shows addresses) and spaces. A name is three to sixteen
ASCII letters and digits with `_` and `-` between them, beginning and ending
on a letter or a digit.

Where a handle appears: every `player` and `host` in the lobby listing, every
`SeatIdentity.display_name` in a `GameStatic` roster, and `/me`. Nothing
*below* the gateway knows a tag exists — the view still carries one string,
and `store::display_names` is the single place the two halves are joined.

`GET /players/{handle}` is how one player finds another: `Alice%23af03`, or
`%23af03` when all that was pasted was the tag. A bare `Alice` is a `400`,
because `af03` is itself a legal display name and a lookup that guessed
between the two would answer differently depending on who had registered
first. It answers `{id, display_name, tag, handle}` — never an address — and
needs a session, because the tags are sequential.

That sequence is a registration counter and the design accepts it: it is the
same thing a BattleTag leaks, and the price of a tag a person can read out
over voice chat rather than a second UUID.

## From an account to a seat

The websocket below is opened with a *seat token*. There is one way to be
given a chair and one way to be given a chair's ticket *again*, and the
difference is the whole of "I closed my laptop". The client walks all of it
itself now (`crates/baylee-client/src/lobby.rs`), which is what turned this
from a curl recipe into a contract:

| step | call | answer |
| --- | --- | --- |
| sign up | `POST /auth/register` `{email, display_name, password, lang}` | `{"ok":true, "confirmation_required":bool}` |
| confirm | `GET /auth/confirm?token=…` (the link in the mail) | `{"ok":true}` |
| send it again | `POST /auth/confirm/resend` `{email}` | `{"ok":true}`, always |
| sign in | `POST /auth/login` `{email, password}` | `{token, expires_at}` |
| who am I | `GET /me` | `{id, email, display_name, tag, handle}` |
| who is that | `GET /players/{handle}` | `{id, display_name, tag, handle}`, `400` without a `#`, `404` for nobody |
| decks | `GET /decks` | `[{id, name, cards, sideboard, commanders, sleeve, playmat}]` |
| one deck | `GET /decks/{id}` | `{id, kind, name, format, description, cards:[…], sideboard:[…], commanders:[…], version}` |
| save a deck | `POST /decks` `{name, cards:["N Card Name"], sideboard, commanders, format?, description?, summary?}` | `{deck_id}` |
| edit one | `PUT /decks/{id}` — same body | `204` |
| throw one away | `DELETE /decks/{id}` | `204` |
| what anybody may play | `GET /decks/shared` | `[{id, kind, name, format, description, cards, sideboard, commanders, version}]` |
| take a copy | `POST /decks/{id}/copy` | `{deck_id}` |
| what it used to be | `GET /decks/{id}/history` | `{version, updated_at, past:[{version, cards, sideboard, commanders, summary, superseded_at}]}` |
| one earlier state | `GET /decks/{id}/versions/{v}` | that state's rows in full, plus `current` |
| put one back | `POST /decks/{id}/versions/{v}/revert` | `{version}` — the **new** number |
| upload a sleeve or mat | `POST /images?kind=sleeve\|playmat`, the image as the raw body | `{id, kind}` |
| fetch one | `GET /images/{id}` | the stored JPEG |
| what a table wears | `GET /games/{id}/cosmetics?token=…` | `{"<seat>":{sleeve, playmat}}` |
| the card pool | `GET /pool?lang=de` | `{total, pool_hash, lang, has_text, cards:[…]}` |
| a card's printings | `GET /printings?card=42` | `{card, english_name, from_catalog, printings:[…]}` |
| tables | `GET /lobby/games?q=&offset=&limit=` | `{games:[{id, name, host, yours, state, seats:[…]}], total, offset, limit}` |
| the same, pushed | `GET /lobby/ws?token=…&q=&offset=&limit=` (websocket) | that page again, on every lobby change |
| open one | `POST /lobby/games` `{deck_id, mode:"ai"\|"open", seats, name}` | `{game_id, seat, seat_token}` |
| sit down | `POST /lobby/games/{id}/join` `{deck_id, seat?}` | `{game_id, seat, seat_token}` |
| take the chair you are in | `POST /lobby/games/{id}/seat` | `{game_id, seat, seat_token}` |
| arrange a chair | `POST /lobby/games/{id}/seats/{seat}` `{kind?, ai?, deck_id?, team?}` | the seat |
| stand up | `POST /lobby/games/{id}/leave` | `204` |

Everything but the two auth calls, `/auth/config`, `/pool` and `/printings`
takes `Authorization: Bearer <token>`. A refusal is `{"error":"…"}` with a
status, and the string is written to be shown to a player as-is — the lobby
does.

**A card row may be written either way a card is printed.** `1 Sheoldred` and
`1 Sheoldred // The True Scriptures` are the same row: the first is what this
pool calls the card, the second is Scryfall's spelling and what a deck site
exports. A **back** face is not accepted and deliberately so — 21 of the 874
two-faced cards have a back that is a card of its own, so `Demonic Tutor`
must keep meaning Demonic Tutor and not the modal DFC behind it. Nor is the
name split: `Lightning Bolt // Anything` is not a card and is refused like
any other name nobody prints.

**A refused card row says which of two things went wrong.** A name is
resolved against the compiled pool, and a miss used to be `unknown card`
whether the name was a typo or Black Lotus. This build compiles 2716 of the
33 694 cards the ledger numbers, so the second case is 92 % of the real cards
a player might type — and `unknown card` is precisely the answer that rules
out what is true, sending them to look for a spelling mistake they did not
make. A name the pool misses is now asked of `baylee-cards-index`, which
numbers every card there is, and a real one answers `that card exists but
this server cannot play it`; the commander field says the same and keeps
`unknown commander` for a name that is nothing. The deck is refused either
way — this reports the problem and does not remove it — and a deck that
imports cleanly never reaches the second lookup at all, because it is only
asked where the first has already failed. `docs/card-identity.md` is
normative, including why it is a second function rather than a widening of
`by_name` and why it reads a two-faced card's name in two tiers.

**A deck has a kind, and only one of the three has an owner.** `account` is
a player's own; `preconstructed` is a retail product; `house` is what this
project publishes to be played with. The database holds both halves of that
with a `CHECK` — `(kind = 'account') = (account_id IS NOT NULL)` — so a
house deck cannot acquire an owner and a player's deck cannot lose one, and
`GET /decks` (yours) and `GET /decks/shared` (everyone's) are two questions
rather than one filtered list. `POST /decks/{id}/copy` is how the second
becomes the first: the copy is an ordinary deck of the caller's own, naming
what it came from as `copied_from` plus `copied_version` — the *state* that
was copied, so it still says what it came from after the original has moved
on. A copy starts with the generated card back, because a sleeve and a mat
are pictures the image store hands out by account.

**A deck's history is what it no longer holds.** The deck row is the
present and carries `version`; `deck_version` holds only states that have
been left behind, so the two can never disagree about what the deck holds
now — and `GET /decks/{id}/history` therefore returns the past *underneath*
a `version` that is the deck itself. A save that changes no list writes no
version. `POST /decks/{id}/versions/{v}/revert` is **not** a rewind: it
saves version *v*'s lists as a new change, archiving the present exactly as
any other save does, and answers the new number. So a revert is revertible
and nothing in a history is ever removed or rewritten — which is also why
`(deck_id, version)` is a primary key with no `ON CONFLICT`: a racing save
fails loudly rather than quietly writing a second past.

`commanders` is a list because of the partner rule (CR 702.124), and the
gateway checks it: at most two, each one a card the rules may seat, and the
pair itself legal under `baylee_cards::decks::may_lead_together`. A `/pool`
row carries `commander` as *eligibility* and no `PartnerKind`, so the
builder can still only name one — the pool row is the thing to widen.

**`POST /lobby/games/{id}/seat` is the way back to a chair you are already
in**, and it is not a join. It names no deck, moves nobody, and changes
nothing another player can see; it asks one question — is this account
sitting here — and answers the same `{game_id, seat, seat_token}` a join
does, so everything downstream of a seat ticket is untouched. It answers in
`waiting` **and** in `playing`, refusing only a game that is `over`, because
a running game is when it matters: the gateway keeps only a seat token's
hash, so a client that restarts has lost its ticket for good, `join` then
refuses with "you are already at this table", and the player is left watching
their own table run without them. Everything else about reconnecting was
already built and all of it hung on that one secret — the engine holds the
chair open (`Deadline::StandIn`, `Session::stand_in`, `SeatAttached`) and the
client re-dials on a schedule (`baylee-client-core/src/reconnect.rs`).

Asking replaces the chair's secret rather than handing out a second one. That
is the right way round: the seat's ticket is whatever was issued last, so a
copy kept by some older client cannot go on answering for a seat its owner
has taken back. The cost is that a player with the table open in two places
keeps only the newer one, which is the same rule a password reset follows.

`/pool` is the deck builder's card list, and one of the two routes with no
account behind it: it is reference data about what this build can play, the
same for everybody, and a sign-in page that cannot show it is worse than a
public one.
Each row carries the registry `index` (the rules identity a saved deck line
resolves to), the printed characteristics, and `coverage`
(`implemented` / `partial` / `unimplemented`) with the author's `note` — a
builder that offered a stub as though it played would be lying. `name`,
`type_line` and `oracle_text` come from the catalog when the gateway has one,
in the language `lang` asks for, falling back field by field to English;
`english_name` never does, because that is what a deck row is written with.
`has_text` says whether rules text was available at all. The whole pool is a
few hundred rows, so it is sent whole and filtered in the client; `total` and
`pool_hash` are there for the day it is not.

Each row also carries `oracle_id` and `alt_names` — every *other* name the
card is printed under: the languages a configured catalog holds, and, for a
card with two faces, its whole `A // B` spelling. That is what lets the
builder show **one row per card** and still find it when a player types the
name on the card in their hand: searching printings instead would list the
same card once per set it appeared in, which is the wrong answer to "do I own
this". `alt_names` is omitted when empty, because for two hundred cards in a
dozen languages it is otherwise the largest field in the response.

The whole spelling is there **without a catalog**, because it comes off the
registry rather than out of an ingest — the same reason `POST /decks` takes
that spelling either way, and the two had drifted apart: the route accepted
`Agadeem's Awakening // Agadeem, the Undercrypt` while the search box could
not find it, which only shows up when somebody uses both halves. One string
also answers the **back** face, because the builder's search is a substring
match: a player who knows that card as the land it becomes types "Agadeem,
the Undercrypt" and finds it, and nothing has to carry that name separately.
A search may do this where a deck row may not — a row has to *resolve* to one
card, so `Demonic Tutor` stays Demonic Tutor there (`docs/card-identity.md`),
while a search *offers*, and showing every card that prints a name is what a
search is for.

Two more flags say what is on a card's **sides**, and they are two because
they are two questions. `has_back_image` is whether Scryfall serves a second
picture for the printing this pool pinned — what the preview and the hover
overlay need, because the URL they would otherwise build answers 404.
`double_faced` is CR 712.1, "a Magic card face on one side and either a Magic
card face or half of an oversized card face on the other. (It does not have a
Magic card back.)", in three kinds: nonmodal, modal and meld. That is what the
builder's `is:dfc` filter means, and it is a claim about the printed card
rather than about what a client can draw. Both are omitted when false.

They differ by the **meld** cards, which are double-faced and have no back
image, because Scryfall keeps a meld back as a card of its own rather than as
a face — 110 against 108 in this pool. One field answered both until it was
found to be neither: it was `def.faces.len() > 1`, a count of what the build
compiled, and it said yes to nine Adventures and two Splits, which print both
halves on one piece of card. Read off the printing's `image_uris` now, and not
off the layout, because an Adventure *can* be printed double-faced and the
layout would then be wrong in the other direction.

`GET /printings?card=<registry index>` is the other half of that trade: the
list stays short by naming cards, and a player who wants a particular piece of
cardboard asks for it. The answer is every printing of that card the catalog
knows — set, collector number, language, rarity, artist, frame effects, and
`finishes`, which is the list `docs/deck-format.md`'s `*F*` / `*E*` may name.
Newest set first.

Without a catalog it is **not** an error: the answer carries the one printing
codegen recorded, in English, plain, and `from_catalog:false`. A picker that
had to handle a `503` here would need a second code path for every gateway
without a database; one that always gets at least one printing does not, and
the deck row it writes is the same row either way. A card outside the registry
is a `404` — the question was about something this build cannot play.

Two distinctions the client has to keep straight. A `401` on a *signed*
request means the account token is spent (sign out and start over); a `401` on
the sign-in form means the password was wrong. And the seat token is not the
account token: it is scoped to one seat at one game, so losing it costs a game
rather than an account, which is why it is the only one that ever appears in a
URL.

`seat` in the answer is a hint. The table states which chair this is, in the
opening payload below, and the client believes the table.

### Sleeves and playmats

A deck carries two pictures: the back its cards are seen from, and the mat its
controller plays on. Both are optional, both are `Deck` fields (`sleeve`,
`playmat`) holding the id of an uploaded image, and a deck with neither is the
ordinary case — the client then draws the *generated* back from
`tabletop::card_back`, which is what `docs/legal.md` §2 requires: the printed
back of a Magic card is somebody's trademark, and arithmetic is nobody's.

Uploading is `POST /images?kind=…` with the picture as the raw body — one
field needs no multipart parser — and it takes an account, so filling the disk
takes an account first. What comes back is a content hash, which makes the
same picture uploaded twice one file and makes `GET /images/{id}` cacheable
forever. An image is stored at exactly one size per kind (a sleeve at 488×680,
the same pixels as a card face; a mat at 1024×512), re-encoded on the way in
rather than served as it arrived — an upload is otherwise a way to hand every
other player at the table an arbitrary file to decode.

The shape is settled **twice**, on purpose. The client's crop tool
(`client-core::crop`) is where a player chooses which part of their picture is
used, and it sends a rectangle already of the target's shape; the gateway then
centre-crops whatever reaches it before scaling, because `resize_exact` on its
own squashes and not every upload comes through that tool. The two agree by
construction rather than by luck: both derive the width from the height with
the same integer division, so a picture the tool cut is its own centred crop
and the gateway finds nothing to do. Two roundings that merely agreed to
within a pixel would still trim a column off every deliberate framing.

Cosmetics never enter the engine. `GameStatic` is rules data, and what a seat's
cards look like from the back changes no rule, so a client asks the *gateway* —
which already knows every seat's deck — with the seat token it is already
holding. `VIEW_VERSION` is untouched by any of this, and a client that does
not ask simply plays with generated backs.

### Which clock a table plays at

Every game this gateway hosted ran the same clock — 600 s to decide and 60 s
to reconnect — because nothing between a room and a `GamePreset` ever wrote
`HouseRules`. **The wire was never the missing part:** the gateway sends the
whole preset to the engine as JSON, house rules included, and gamehost has
always decoded them. What was missing was a way to say which one.

`POST /lobby/games` takes `clock` (a name) and, overriding it,
`decision_timeout_secs` and `reconnect_window_secs`. Nothing said is `casual`,
which is exactly what every table played at before, so no existing caller
changes behaviour. There is no `custom` sentinel: a name picks a row and a
number replaces one field of it, so "blitz but longer to come back" needs no
fifth preset.

| name | decide | reconnect | |
| --- | --- | --- | --- |
| `casual` | 600 | 60 | the default, and the old behaviour |
| `standard` | 120 | 60 | |
| `blitz` | 30 | 30 | |
| `untimed` | 0 | 60 | no decision clock at all |

`GET /auth/config` publishes this table with a one-line blurb each, so a
client builds its picker from what the gateway accepts rather than from a
copy that goes stale.

**Zero is not symmetric, and that is deliberate.** A `decision_timeout_secs`
of zero is a *choice*: the engine reads it as no deadline (`clock()` returns
`None`), which is the only way to play a game that cannot be lost on time. A
`reconnect_window_secs` of zero is **refused**, although the engine would
accept it — with no stand-in clock a seat whose player closed their laptop is
on no clock at all and the whole table waits on them forever, which is the
exact failure `reconnect_window_secs` was added to end. That is a gateway
policy about hosting strangers, not a rules one; a local harness may still
choose it. Anything between 1 and 9 seconds is refused on both, and an hour is
the ceiling: under ten seconds is not a fast game, it is one nobody can read a
board in.

**The limit is in the listing and not in the game.** Every lobby row carries
`clock: { decide_secs, reconnect_secs }`, because a player choosing a table is
choosing a pace and finding out by losing a decision is not a choice. It is
deliberately *not* sent to a seat during play: the in-game warning is drawn
from the remaining time alone, so a thirty-second table does not get a clock
for three seconds. A room has no preset until it starts, which is why the
number lives on the lobby model rather than being read back off one.

A rematch inherits the clock. Pressing *play again* at a blitz table is a
request for another blitz game, and there is no screen between the button and
the new room on which anyone could have said otherwise.

### Rooms

A table with more than two chairs is a **room**, and the whole of it is
arranged before anyone plays. `POST /lobby/games` with `seats: 2..=8` opens
one; the host takes the first chair and every other chair starts open. `name`
is what the table is called in the list, and may be empty — the listing then
falls back to the host's display name. Eight is `GamePreset::validate`'s own
bound, so a room the gateway opens is never one the engine would then refuse
to build.

`password` locks the room: a non-empty one is stored as a SHA-256 hash and
every `POST …/join` has to carry it or get a `403` — checked before anything
else about the room, so a stranger with the wrong password cannot learn how
full it is. Argon2 guards an account; this guards a table for an evening and
is checked on every join, which is a different trade. The listing says only
`"locked": true`.

`GET /lobby/games` describes a room completely, because deciding whether to sit
down means seeing what is already at the table. It answers **one page**:

```json
{ "games": [ … ], "total": 31, "offset": 8, "limit": 8 }
```

`q` matches a table's name and its host's display name, case-insensitively and
anywhere in either; `waiting_only` drops the games already being played;
`offset` and `limit` (25 by default, 100 at most) cut the page out. `total` is
what the search matched, not what the page holds — a pager with no idea how
many there are is a Next button that has to be pressed to find out it does
nothing.

The **order is fixed and total**: waiting rooms first, then newest first, then
by id. Games live in a `HashMap`, and paging an unordered collection hands out
some rows twice and never shows others — which no single page ever looks wrong
enough to reveal.

One game in that page reads:

```json
{ "id": "…", "name": "Kitchen table", "host": "viktor", "yours": false,
  "state": "waiting", "locked": false, "startable": true, "rematch": false,
  "seats": [ { "seat": 0, "kind": "human", "ai": null, "taken": true,
               "player": "viktor", "you": false, "host": true,
               "deck": "Mono-Green", "ready": true },
             { "seat": 1, "kind": "ai", "ai": "sharp", "taken": false,
               "player": null, "you": false, "host": false,
               "deck": "", "ready": true } ] }
```

Never an account id — a `player` is a handle (`Alice#af03`), and `you` / `yours` answer
"is that me" without the listing having to carry anyone's account. `kind` is
`"human"` or `"ai"`, and `ai` names a difficulty from `AIProfile::NAMED`
(`novice`, `casual`, `steady`, `sharp`, `expert`); one that does not exist is a `400` rather than a
quiet default, because a table that plays at another level than it advertises
is worse than one that says no.

Two authorities, and they do not overlap. The **host** arranges the table —
`POST …/seats/{seat}` with `kind`, `ai` and `team` — but not a chair someone is
sitting in (`409`). Every **player** sets exactly one thing, the deck they
themselves will play, through the same route with `deck_id`; a deck that is not
theirs is a `403`, and so is any attempt to arrange a seat that is not their
own.

**Sides are the host's**, and that is why `team` sits with `kind` rather than
with `deck_id`: a side is the format, and a format the people at the table can
change between them is not one — a player who could pick their own would pick
the winning one. Teams are numbered from 1, `0` puts a chair back on its own
side, and the listing carries `"team"` per seat (`null` for a chair playing for
itself, which is every chair at a table with no teams on it). Moving a chair to
another side clears that chair's `ready`, exactly as swapping its deck does:
it is not the game they said yes to. A team need not be balanced — 3v2 is a
table — and the one arrangement refused is everyone on the same team, which
`POST …/start` answers with a `409` because `GamePreset::validate` refuses it
and the lobby refuses exactly what the engine would.

The rules half is in `docs/engine-internals.md`: an opponent is a *side*, not a
seat, so teammates cannot be attacked, are not "each opponent", and are not
stopped by hexproof; and the game ends when one side is left standing, with
`GameEnded.winners` carrying the whole winning team rather than the one seat
that survived.

**Starting takes two statements by two people.** `POST …/ready {ready}` is a
player saying they are ready, and only ever about their own chair — a `409` if
they have no deck yet, and reset by the host putting a different deck in that
chair, because a deck they have not seen is not one they said yes to. An AI
chair needs none of this: it is ready as soon as it is configured, and one the
host gave no deck plays the house deck. `POST …/start` is the host's go, a
`403` for anyone else and a `409` while any chair is not ready; `startable` on
the listing is that same condition, published so a player can see who
everyone is waiting for.

A room used to start itself the moment the last chair had a deck in it. That
read well until "ready" and "has a deck" stopped being the same sentence:
picking a deck to look at it put you in a game.

`POST …/host {seat}` hands the room to whoever is sitting in that chair — by
seat rather than by name, which is the one handle that stays unambiguous when
two players share a display name. `403` for anyone but the host, `409` for an
empty chair.

`POST …/leave` frees a chair, and a room outlives its host: it passes to the
player who **joined earliest** — arrival order, not seat order, because chairs
are taken in whatever order people pick them — and only a room with nobody
left in it is closed. Closing it the moment the host stood up, which is what
happened before, threw everyone else out of a table they were sitting at.

**A seat token is not always usable yet.** `mode:"ai"` and a join both order an
engine before answering, so the socket can be opened at once — it simply waits
(up to 30 s) for that engine to attach before the first frame arrives. That
readiness is *sticky*, and has to be: with a warm engine binary the attach
happens within a few milliseconds of the order, well before the player who
placed it has finished dialling, and a socket that could only be told as it
happened would sit out the whole timeout waiting for something that had
already arrived. An
`"open"` table orders nothing: it holds the seat and waits for a second player,
and a socket opened against it is accepted and then closed with nothing on it,
because there is no game yet to describe. The host of an open table has to wait
for its `state` to turn `"playing"`, which is what the lobby feed below is for.

### Playing it again

`POST /lobby/games/{id}/rematch` answers the `{game_id, seat, seat_token}` a
join answers — a ticket to a table that is not the one in the path. `403` for
anyone who was not sitting at that table; a game's id was in every player's own
listing while it waited, so it is no secret and cannot be what the check rests
on.

`{id}` is **either end of the pair**: the game that is over, or the room opened
from it. The two people pressing the button are looking at different things —
whoever just finished has the game's id on the screen in front of them, and
whoever went back to the lobby has the room's, the finished game being in
nobody's listing. `409` for anything else.

The route is **idempotent**, and that is the whole of it. The first press opens
a room with the arrangement copied — how many chairs, the sides they play for,
who was in them, which were the AI and at what difficulty, the name and the
password — and writes a pointer to it on the finished game. Every press after
that follows the pointer, so two people pressing at the same moment sit down at
one table instead of at two. A human chair's deck is re-read from the store, so
an edit between games counts; what the finished table played is the fallback,
for a deck deleted since.

Pressing it is also the ready statement, because `said_ready` means "I want to
play" and there is nothing else a button that says *play again* could be saying.
So the room starts itself once every chair is ready. That is not the rule taken
out of `set_seat` above: that one fired when a player picked a deck to look at
it, and this one fires when everyone at the table has asked for another game.
An AI chair is ready as soon as it is configured, so a solo player is one press
from the next game — which is the case that matters most, the game being
repeated having itself been one tap.

Nothing of the finished game travels. The seat token is new (one token names one
game for the whole of its life), the preset is rebuilt from the chairs when the
room starts rather than inherited, and `said_ready` is written only for the
player who actually pressed. The other players learn of the room the ordinary
way: it is a waiting table in their own listing with their chair already in it,
`you: true` — the finished game itself is filtered out of every listing and has
nothing to say. The one thing the pointer does buy is a stay of execution: a
finished game whose room is still waiting outlives its grace period, or the
player who takes a minute longer to press would open a second table.

A copied chair is **reserved, not taken**. It carries the account that had it
and the deck it played, and no seat token, because a token can only be minted
into a reply to the player it belongs to — so a chair is not `ready` until it
has one, and `POST …/ready` from the lobby says the right thing without being
enough. Every other way of taking a chair issues the token in the same breath,
which is why the rule reads as noise everywhere else: a table that started on a
reserved chair would be a table its own player could not open a socket to.

Which is why the listing says `"rematch": true` on such a room. It is the one
thing a client cannot work out for itself: the row looks like any other waiting
table with its chair in it, and the button on it has to be *play again* rather
than *ready*. A client that sent the wrong one would be answered `200` and
would then draw the chair still saying `"ready": false`, which is the truth and
no help at all.

### The lobby feed

`GET /lobby/ws?token=<account token>&q=&offset=&limit=&waiting_only=` is a
websocket carrying **the page that socket asked for**, sent once on connect and
again on every change to the lobby: a table opened or closed, a chair taken,
freed, arranged or readied, a room started, a game ended. The token is the
account bearer token in the query string, because a browser cannot put a header
on a websocket; an unknown one is a `401` on the upgrade itself.

The payload is the same object `GET /lobby/games` answers, rendered for *this*
reader — `yours`, `you` and `player` are per-account, so the fan-out is a
notification, and each socket then renders its own page. A subscriber that
falls behind is not replayed: every frame is the whole page, so the newest one
is the only one worth having.

Nothing is sent up the socket. A change of search or page is a **different
subscription**, so the client closes it and dials again with the new query;
that keeps the socket's answer and the HTTP route's answer the same question,
asked over two transports.

`GET /lobby/games` remains, and remains the fallback: a client with no socket
polls it, which is what the two-second re-read used to be for everybody.

## The opening payload, and a client that is not the server

`GameStaticMsg{game_id, view_version, static_json}` is the one message a
networked client cannot do without and could not previously receive. The seat
roster and the print table are built from the `GamePreset`, which a client
never sees — so `LocalHost` built them out of the preset it happened to be
holding, and nothing on the wire carried them. A socket client had a print
table of length zero and every `PrintRef` named no card.

`Session::game_static_envelope` now produces it and it is always the first
thing on a seat's wire: `EngineRunner` sends it on `SeatAttached` (and again on
a reconnect, since that is a new socket), the engine-server's dev harness on
`CreateGame`, `Join` and `Resume`. `LocalHost` takes it off the wire too, through the same
`host_message` decoder the networked host uses — a field only the in-process
path filled in would be missing in exactly the case nobody tests at a desk.

`view_version` is duplicated outside `static_json` on purpose: a client that
cannot render this version must be able to say so without first decoding the
structure whose shape is what changed.

Sending it exposed the one hidden-information leak the view design had no
field for. The print table is deduplicated **per card across the whole game**,
so `prints` is the union of every decklist at the table — a seat that received
it could subtract its own deck and read the opponent's. `prints` is therefore
`Vec<Option<PrintEntry>>` now, filled in per seat: `own_prints` seeds a seat
with the printings of its own deck, sideboard, opening hand and starting
battlefield, and `Session::reveal` marks the rest as the seat's views show
them, re-sending `GameStaticMsg` **before** the view that points at the new
entry. A hole rather than a shorter list, because the index is the `PrintRef`
every object in every view carries. That is **`VIEW_VERSION` 6 → 7**.

What stays readable is the table's *length* — how many distinct cards are in
play across every deck, and therefore how many the opponent plays that you do
not. That is a deck-diversity number, not a decklist, and hiding it would mean
padding the table with entries no object points at.

`GameStatic::print()` already returned `Option<&PrintEntry>`, so almost no
client code changed; what did change is that `textures.rs` no longer preloads
the whole table (it was prefetching art for every card in the opponent's deck)
and `cardtext.rs` re-asks the catalog when the table grows.

`Session::describe(game_id, names)` supplies the parts the rules kernel has
never heard of. `Session::snapshot` deliberately reveals nothing: it rebuilds a
state a `pump` already showed that seat, so the printings were earned there.
The gateway's lag resync does re-send the payload around it, because the update
that granted a printing travelled the very broadcast that socket just dropped
messages from.

`NetworkHost` (`crates/baylee-client/src/net.rs`) is the second
`DuelHost`. It is handed a `SeatTicket{gateway, game_id, seat, seat_token}`
and connects to `/games/{id}/ws?token=…`; `poll` drains the socket without
blocking and `submit` sends a `PlayerActionMsg`. The token is *not* repeated
in each frame — the socket is already bound to one seat of one game, and the
seat comes from the token rather than from anything the client says later.
The seat in the ticket is only a hint: the host believes `GameStatic.your_seat`.

`reconnect()` re-dials and sends `ResumeGame{last_seq}`. It is deliberately
not automatic: only the application knows whether a player is still sitting
there, and a host that redialled by itself would hammer a gateway that is
down.

`ewebsock` is the transport for the same reason `ehttp` is the HTTP client —
one API that is a background thread natively and the browser's own
`WebSocket` on wasm. The browser's socket handle is neither `Send` nor
`Sync` and a Bevy resource must be both, so on wasm it is wrapped in
`SendWrapper`: a run-time thread check that panics rather than an
`unsafe impl` that would be a claim nobody could enforce.

Where a ticket comes from: `BAYLEE_GAME` + `BAYLEE_SEAT_TOKEN` in the
environment natively, `?game=…&token=…` in the page URL in a browser — which
is how a web lobby hands a player to the table. Without one the client plays
solo against the house AI, in process, exactly as before. A ticket that is
present but unusable is a hard stop, not a quiet fall back to solo play:
somebody is waiting at that table.

Still not done here: the client has no lobby of its own. Logging in, listing
games and picking a deck are HTTP calls somebody still has to make; the
client only knows what to do once it has the ticket.

## Attacking a planeswalker (view version 3)

`PlayerAction::DeclareAttackers` carries `(ObjectId, Defender)` pairs
rather than `(ObjectId, PlayerId)`, where `Defender` is
`Player(PlayerId)` or `Planeswalker(ObjectId)`. `Pending::ChooseAttackers`
now carries the legal `defenders` list, and the engine validates a
declaration against exactly that list — "which planeswalkers may I attack"
(CR 506.2) is a rules question, and a client re-deriving it would be a
second, divergent implementation of the rule. `AttackerView::defending`
changed the same way, which is what took **`VIEW_VERSION` from 2 to 3**;
a client checks that on `HelloAck` and refuses a host it cannot render.

`Defender` is an externally tagged serde enum over two transparent ids, so
the JSON is `{"Player":0}` or `{"Planeswalker":1234}` where a v2 payload
had a bare `0`. A v2 client therefore fails to deserialise rather than
silently reading a planeswalker attack as an attack on seat 0 — which is
the point of the version bump being mandatory rather than advisory.

`ResumeGame{last_seq}` is answered as of 2026-08-31, by both servers.
`Session::resume` is deliberately **read-only**, where `pump` advances the
game: rebuilding a client through `pump` would drive every AI seat forward
as a side effect of somebody reconnecting. A client that is already current
(`last_seq >= seq`) is sent nothing rather than a redundant re-render. The
gateway now also uses that snapshot when a seat's socket lags behind the
broadcast, instead of dropping the player.

`HouseRules::decision_timeout_secs` is enforced as of 2026-08-31. It was
carried from the preset through `baylee-gamehost` into the proto and read by
nobody. The clock lives in the **engine process** (it started in the gateway;
it moved with the rules), never in the rules kernel or the session: the kernel
is deterministic, and a session that timed itself would replay differently on
every machine. `Session` only answers three clock-free questions — who owes an
answer, what a legal answer would be, and how many questions have been asked
(`decision_seq`) — and the process anchors a deadline to that last one so it
restarts when the game moves rather than whenever the task wakes up. On
expiry the house agent answers for the seat, because it is legal for every
`Pending`; a timeout that produced an illegal action would leave the same
seat stuck on the same question forever.

`HouseRules::reconnect_window_secs` is enforced as of 2026-09-05, and it is a
**second clock**, not a longer first one. A seat with no socket is on no
decision clock at all — nobody should lose on time to a question they never
saw — which was right and left the table waiting on a closed laptop forever.
So the same `awaiting_seat()` is on exactly one of two deadlines: the decision
clock if its socket is here, the reconnect window if it is not, and neither if
the chair is an AI's (it never had a socket to lose). The two limits are
independent: a table that gives its players all the time in the world still
must not sit forever on a player who has gone.

They also expire into different things. A decision timeout answers *once*,
because the seat is there and simply took too long over this question. A
reconnect timeout changes *who answers*: `Session::stand_in` puts the house in
the chair, because a player who is not there for this question is not there
for the next one either, and answering once would put the table back where it
was one decision later. `SeatAttached` hands the chair straight back
(`Session::hand_back`) — no window, no second deadline — and both directions
re-check the race, since a socket can return between a timer firing and the
expiry being applied.

It is the mirror of `SeatKind::Driven`, and the two are why a chair and
whoever is answering for it are separate questions in `Session`. A held chair
is **not** relabelled as an AI: `SeatIdentity` gained `away` beside `is_ai`
(`VIEW_VERSION` 12), because a seat that renamed itself to the house after a
thirty-second hiccup would be telling the table something untrue, and would go
on saying it after the player was back. The roster travels in `GameStatic`,
which is sent once per socket, so a chair changing hands now marks every
seat's roster stale and the next view carries a fresh one — which had been
missing since `take_over`/`release` existed.

Still open here: `time_extension_votes`, which unlike everything above does
need the wire — `TimeExtensionRequest/Vote/Result` are not in the proto.

Server: `baylee-engine-server` (tokio + tokio-tungstenite), one process,
games as `Session`s — engine + human seat + auto-driven AI seats
(baylee-ai). E2E smoke: real binary over a real socket
(`crates/baylee-engine-server/tests/e2e.rs`).

On the **listening** harness a socket names its own seat: `seat_token` is a
seat number, because that socket authenticates nothing and the field would
otherwise be read by nothing. Naming an AI chair takes it over
(`Session::take_over`), so the questions the house AI would have answered go
out over the wire instead, and hanging up gives the chair back. Two rules
keep it honest — a seat another socket is already driving is refused, and a
seat that is not at the table is refused — and the reason it is loopback-only
is now larger than it was: a connection is handed the hidden information of
whichever seat it names.
