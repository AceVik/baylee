# Observed faults — from live play, 2026-09-08

Reported by the project owner while playing, plus what measurement has and has
not confirmed. Nothing here is fixed. An entry is promoted out of this file
when it has a test that fails for the right reason.

The point of the file is that a fault reported from a real game is worth more
than one found by reading, and it is also the easiest kind to lose.

## Client

### 1. Card images not loading — NOT REPRODUCED

Reported: "Kartenbilder werden nicht geladen/angezeigt."

Measured instead: on the offline path (`house_duel`, no gateway) seven cards
in hand rendered with full art and printed text. The gateway's own route is
sound — `GET /art/small/front/3/a/<uuid>.jpg` answered `200 image/jpeg`, and a
cache miss made the gateway fetch from Scryfall itself.

Two hypotheses died on the way and are recorded so they are not chased again:

- The nil-UUID guard in `images::image_url` looked like the cause; its own
  comment says it exists for "a preset with no real print table". The preset
  that carries `Uuid::nil()` is behind `#[cfg(test)]` (`host.rs`), and the real
  offline preset goes through `decks::reference_print`, which uses the card
  definition's actual `scryfall_id`.
- `board.rs::a_board_resolves_end_to_end_into_fetchable_image_urls` already
  proves the model resolves a printing to a fetchable `.jpg`.

Still needed: the conditions under which the owner saw it. A gateway game, the
deck builder, and the table are three different paths.

### 2. The art base is set on one path only — CONFIRMED, unreported

`images::use_art_base` is called in exactly one place,
`baylee-client/src/lobby/http.rs`, from the `/auth/config` callback and only
when the gateway answers `art_cache: true`. Every other way into a duel —
offline play, and a client launched with a `SeatTicket` through `BAYLEE_GAME` /
`BAYLEE_SEAT_TOKEN` — leaves `ART_BASE` unset, so `art_base()` falls back to
`SCRYFALL_CDN` and the client fetches from Scryfall directly.

It works, which is why nobody noticed. It also defeats the reason the mirror
exists: `art.rs` opens by explaining that four seats at a table otherwise pull
the same pictures four times, and `docs/legal.md` §3 puts the rate limit on the
gateway for the same reason.

### 3. A fetchland puts mana in the pool instead of searching — NOT REPRODUCED

Reported: "Fetchland Effekt fügt irgend was in den Manapool statt mich ein
passendes Land aus der Bibliothek auswählen zu lassen."

The card is not the fault. `crates/baylee-cards/src/cards/flooded_strand.rs` is
an `activated!` with `CostPart::TapSelf`, `SacrificeSelf`, `PayLife(1)` and
`Effect::SearchLibrary { finds: &[Find::BATTLEFIELD] }`. There is no mana
ability on it.

So the fault is in the client's click path. The two shapes that would produce
exactly this symptom: a land treated as a mana source without reading its own
`mana_ability` flag (CR 605.1 makes a mana ability the exception, not the
rule), or a `ManaRun` owning the click. Reproducing it needs a fetchland put in
hand deliberately rather than drawn.

## Engine

These four came from one game and have not been investigated yet.

### 4. "Mein Mana ist verschwunden"

Floating mana lost when it should have been held. Needs the step it emptied at
— mana empties at the end of each step and phase (CR 500.4), so the question is
whether it emptied *early*.

### 5. "Beim Gegner wurde das Mana nicht enttappt"

Read as: the opponent's lands were not untapped in their untap step.

### 6. An ability whose source was exiled in response

Ondu Cleric's enter trigger was on the stack; Path to Exile exiled the Cleric;
the ability did not do what the owner expected. CR 608.2 is the rule that an
ability on the stack is independent of its source.

### 7. Path to Exile gave no life — MISATTRIBUTED, and it uncovered a real one

Reported as "hätte mir 1 LeP geben sollen (wegen Ondu Cleric, 1/1)".

Path to Exile gives no life to anybody. Its printed text is "Exile target
creature. Its controller **may search their library for a basic land card**,
put that card onto the battlefield tapped, then shuffle", and
`cards/path_to_exile.rs` implements exactly that
(`Effect::OptionalBasicLandSearchFor { player: ControllerOfTarget }`). The life
in this interaction could only have come from the Cleric's own trigger.

And that trigger resolving to nothing is **correct**. Ondu Cleric gains life
"equal to the number of Allies you control", and `CountOf` is evaluated when
the ability *resolves*. Path exiled the Cleric first, so at resolution the
count was zero. The ability did resolve; it resolved to nothing. Reported
faults 6 and 7 are one event, and the engine got it right.

What this did uncover is a genuine divergence, unrelated to the report.
The oracle text is "**you may** gain life equal to the number of Allies you
control" — the "may" is a choice the player is entitled to make, and
`cards/ondu_cleric.rs` implements an unconditional `Effect::GainLife`. There
is no optional-effect primitive in `baylee-cards-dsl` at all, and Ondu Cleric
is the only card in the pool whose text contains "you may gain". So this is
not a typo in one card: it is a hole in the DSL that one card is currently
sitting in, and `xtask validate` did not catch it because the header and the
`CardDef` agree — they are both missing the same word.

Two consequences worth separating. Gaining life is almost never something a
player wants to decline, so the *practical* cost is near zero; the cost that
matters is that the rule "a generated or hand-written `Implemented` means the
card does what it prints" is not currently true here. Either the DSL learns to
express a may-effect and the card uses it, or the card drops to
`Coverage::Partial` and says why.

### 8. The untap step reads correct — needs the exact turn

"Beim Gegner wurde das Mana nicht enttappt."

`progress.rs::untap_step` untaps permanents whose `controller` is the **active
player**, which is what CR 502 describes: a player untaps their own permanents
in their own untap step. An opponent's lands staying tapped during *your* turn
is therefore right, and only staying tapped through *their own* untap step is
a fault.

The report does not say which it was, and the two look identical on screen if
the phase rail is not being read. Needs the turn and step.

## Second play session, 2026-09-08 evening

Reported by the owner after a longer game. Grouped by where the work lands,
not by the order they were told, and nothing here is fixed yet.

### Client — frame and flow

9. **"You have priority [OK]" reads as a gate.** The prompt looks like a modal
   that must be dismissed before play can continue. It is not, and that is
   exactly the problem: an acknowledgement button on a window that grants
   rather than asks. The phase controls already show what a non-blocking
   affordance looks like.

10. **The phase rail stops at steps nothing can happen in.** Untap, upkeep,
    draw, damage and cleanup should not be default stops. Cleanup may not
    belong in the rail at all — a player gets priority there only when a
    trigger or a discard puts them there (CR 514.3a).

11. **Too much confirmation.** "Tap to draw" should be one gesture: the card,
    the effect, done. The two-stage arming line is drawn in the wrong place
    for abilities whose whole cost is a tap.

12. **Hand order.** The hand must keep the order the cards were drawn in.

### Client — zones and the board

13. **Graveyard and exile are not readable.** Both should be visible at all
    times, and clicking a pile should open that zone in the search dialog —
    sortable, searchable, scrollable.

14. **Creatures lay out badly, and tokens make it worse.** Overlaps that
    should not happen once tokens are on the table.

    *Fixed.* A card taps by turning a quarter of the way round, so it claims
    its long side of a row and not its width — and `pack_lane` packed to the
    width, so every tapped land and every attacking creature sat 0.14 units
    inside each of its neighbours on a row with seventeen units to spare.
    Tokens only made it louder: more cards, tighter pitch, the same fault.
    Every cell is `CARD_SPAN` wide now, so nothing overlaps and tapping moves
    nothing — the cell was always the right size and the card turns inside it.

15. **Tokens render ugly.** They should carry pictures the way Forge's do.

16. **A copy has no provenance.** A permanent that entered as a copy of
    something else says nowhere what it copied, and the copy's own abilities
    were not offered ("tap: draw a card" on a copy that has it).

17. **Copy tokens are indistinguishable from the real card.** A token needs a
    mark that says token.

18. **A creature entering as a copy from another card's effect** arrives on
    the stack with no explanation of why.

19. **The first land played vanished, reappeared and vanished again** — while
    still being counted. A rendering or lane-packing fault, not a rules one.

    *Fixed, and it was the lane packing.* Identical permanents collapsed into
    a counted card on every board rather than only on a full one, so the
    second Forest swallowed the first; tapping one for mana split them apart
    again, the summary key carrying the tap; untapping put them back together.
    Nothing was ever miscounted, which is exactly why the count stayed right
    while the card was gone. The collapse is gated on the row actually running
    out of space now. Its one visible cost: playing the fourteenth land on a
    duel's row snaps thirteen cards into one stack in a single frame, which
    `Motion` glides.

20. **Target selection needs a real design.** Attacking and every other
    "choose a target" step.

21. **Reanimation works but does not flow.**

### Engine and cards

22. **Equip does nothing.**

23. **"Spend mana as though it were mana of any colour" has no effect.**

24. **A card with Waterbend 6 is broken outright** — it asked for 99 targets,
    cost 5 mana, could not be given targets, answered "invalid targets or
    costs", and left the game unplayable from that point. The worst of the
    lot: it ends the game, not just the interaction.

    *Three separate faults, two of them fixed.* Waterbend is an optional
    additional cost paid convoke-style, so the cast asks "waterbend {6}?" and
    then "tap what you like to help pay"; taking the first and declining the
    second leaves `{10}{U}` against five lands, which the engine has to
    refuse.

    - **Fixed.** The refusal walked the priority round on to the *next seat*
      — `advance_cast_wizard` resumed through `run_until_choice`, and to
      `priority_round` a holder who is no longer being asked has taken their
      turn. CR 601.2h reverses the whole casting, priority included. That is
      the whole of "unplayable": `EngineServer::refused` re-sends the
      refusing seat's snapshot, and `Session::snapshot` puts the choice in
      front of the seat being *awaited*, so the caster got a table with no
      question on it while the seat that held the question was never pumped.
    - **Fixed.** "99 targets" was literal: the convoke stage published
      `Pending::ChooseTargets` with `max: 99` as a sentinel. It carries a
      `TargetPrompt` now, is bounded by what is on the table, and reads "tap
      creatures or artifacts to help pay". Delve had the same two faults and
      got the same two fixes.
    - **Open.** `spirit_water_revival.rs` declares plain `convoke: true`, so
      the tapping is offered on the un-waterbent `{4}{U}` cast as well.
      Waterbend's reminder text scopes it to the waterbend cost alone. That
      needs a DSL that can say "convoke, but only for this additional cost".
    - **Open, UX.** The kicker is offered as a yes/no with no indication that
      answering yes cannot be paid. A player should not be able to walk into
      a refusal the client could see coming.

25. **Manual taps must stay possible** alongside the auto-tap that already
    works well.
