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

    *Fixed for the tokens the registry defines.* A token has no printing, so
    the board model asked for no image at all and the renderer fell back to
    drawing the card's own face — which is what a flat coloured rectangle
    with a name across it was. An `ImageKey` now names a `Print` **or** a
    `Token`, the id being the one the engine already stamps on the object,
    and each of the fourteen `TokenDef`s carries the Scryfall id of a printed
    token card. Chosen one at a time rather than by search, because the
    picture carries the token's printed text: the Angel is the Shadows over
    Innistrad 4/4 with flying and *not* one of the many 4/4 Angels with
    flying and vigilance, and the Army is the Lord of the Rings Orc Army,
    since no card prints a bare "Army".

    Still bare: a **copy** token. It is a copy of a card rather than of a
    registry token, so it carries no token id and there is nothing to look
    up. That is entry 16 below, not this one.

    The gateway's mirror needed no change to serve them: `/art` is keyed on a
    well-formed id and the two shards derived from it, never on membership of
    a print table, so a token id it has never seen is fetched and cached like
    a printing. Worth writing down rather than assuming — a route that *did*
    check the table would have 404ed every token on the lobby path while
    working perfectly under `dev-table`, which fetches straight from the CDN
    (entry 2), so the fault would have been invisible to every test made here.

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
    out of space now. Its one visible cost is at the threshold: playing the
    fourteenth land on a duel's row snaps thirteen cards into one stack in a
    single frame, which `Motion` glides — and a board sitting *at* thirteen
    flips between the two every time a token dies and comes back, which is
    the version a player will actually notice.

    The gate made `pod_width` load-bearing, and it was wrong: one number for
    the whole table, read off the *first opponent's* row. That is only ever
    right on a table nobody has focused, where seats divide the ring evenly.
    Focusing an opponent widens that seat and shrinks the others — measured
    at two seats: 14.70 units for the local pod against 23.71 for the focused
    one — so the local board was being gated against a row half again its
    size, and at three seats or more every unfocused pod was gated against
    whichever seat happened to sit at ring index 1. `BoardModel::from_view`
    takes a width *per seat* now.

20. **Target selection needs a real design.** Attacking and every other
    "choose a target" step.

    *Designed, not yet built.* `docs/redesign-proposal.md` §10 is the design
    and §10.7 the six commits it lands in. One model carries all of it: a
    pick is a pair — a thing, and what it is pointed at — so an attacker
    against a defender, a blocker against an attacker and a target against
    its spell differ only in *who supplies the second half*, which is a fact
    the engine already gives us in each case. What the engine offers is lit
    with the perimeter light, which is free precisely while a question stands
    (`legal_actions()` is `None` outside priority, so nothing is activatable
    then); what it does not offer is drawn exactly as it always is, because a
    felt that darkens under every spell says the same thing twice. `Space` is
    the one send at every count and `Esc` takes back one pick.

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

## Third pass, 2026-09-08 — from the screenshots

Two faults seen in a four-player table photographed for a friend. Both are
about the table itself rather than the rules, and both are fixed.

26. **Cards on the battlefield z-fight.** *Fixed.* Not precision: the two
    quads were at *exactly* the same height. Every card on a lane was placed
    with a lift of `0.0`, so a fanned row — which is overlap by definition —
    left the depth test deciding per pixel, on the last bit of an
    interpolated float, and it decided differently as the camera moved. The
    covered card's art came through the one on top in bands. A row now rises
    by `LANE_RISE` from its first card to its last, shared out over however
    many cards are on it so a long lane cannot ramp; the whole rise is
    smaller than the gap a card already floats above the felt, and the order
    it imposes is the one a fan wants anyway — each card over the one before
    it.

    Why a four-player table is where it showed: a lane fans as soon as it
    holds more than it has room for, and that was about *six* permanents at
    four seats, because of entry 27.

27. **A seat's board is too narrow at four players.** *Fixed.* The ring was
    sized by inverting the arc formula for `MIN_POD_WIDTH` — a closed form
    for a quantity no seat is ever handed. What a seat actually gets is
    `pod_half_width`: the tighter of that arc and the true distance to its
    nearest neighbour (the ring is an *ellipse*, and the seats on the flanks
    sit far closer than a circle of the same mean radius puts them), less a
    pile strip on each side. So the solve aimed at 10.0 and delivered 6.10 at
    four seats, 4.01 at five and 3.31 at six — four cards on a row whose
    constant promises seven, and five seats came out narrower than six.

    The ring is now solved by bisection over `pod_half_width` itself, which
    is the property that was missing: the search asks the same function the
    width is read from. Measured at the duel HUD's aspect, four seats go from
    6.10 to the full 10.00 and five from 4.01 to 7.86. Two ceilings bound it
    (`MAX_RING_X`, `MAX_RING_Y`), because past them `CameraRig::home` runs
    into `MAX_DISTANCE` and the near mats slide under the hand bar — which
    the old layout was already doing at five, six and eight seats on a
    portrait canvas. Crowded tables stop there and fan, which is what fanning
    is for.

    *Open, and new:* on a canvas taller than it is wide the ceiling now binds
    from three seats up, and a pod there comes out about two units — a fifth
    of the promise. It is not a regression (the old layout clamped the camera
    at exactly `MAX_DISTANCE` in the same cases, which is the failure the
    ceilings exist to stop), but a portrait table is a fanned table, and the
    answer for it is probably a different arrangement of seats rather than a
    bigger ring.

    Entry 29 then took five seats the rest of the way, to the full 10.00, and
    moved the portrait bound out with it: at aspect 0.60 a table of three now
    gets 9.8 where it got 2.0, and four gets 7.9. Five and more still fan.

28. **Every seat but the near and far ones had its board back to front.**
    *Fixed.* At four players, seat 1 sat at `(12.53, 0)` with its creature
    lane at `(14.17, 0)` — *further* from the middle than the seat itself —
    and its lands at `(10.88, 0)`, facing the channel. Its library stood on
    its left hand. The whole board was mirrored, and combat happened behind
    the player's back.

    The ring placed a seat at `(rx·sin θ, −ry·cos θ)`, which walks
    anticlockwise from the near edge, while every frame built out of
    `facing` — `away = (sin f, cos f)` in `lane_center`, `side = (cos f,
    −sin f)` in `pile_center` — is a *clockwise* rotation. The two agree
    exactly where `sin θ = 0`, which is to say at the near seat and the seat
    opposite: a duel is right, and nothing else is. The one test on lane
    order asked the local seat of a two-player table.

    The centre formula is negated now, which fixes the frame and the seating
    order together: the next player in turn order sits on the **left**, as
    the module doc always said and as Magic's clockwise turn order means at a
    real table. `seats_are_ordered_clockwise_in_turn_order` used to assert
    the opposite; `lanes_stack_from_the_table_centre_towards_the_seat` now
    asks every seat of every table from two to eight, at two aspects.

29. **The flanks of the ellipse were far tighter than the rest of the ring.**
    *Fixed.* Sides were spaced by the angle that parameterises the ellipse,
    not by distance along it, and on a wide ring those are nothing like the
    same thing: at six seats on a 19.9 × 11.2 ring the two flank seats sat
    11.2 apart while the near seat had 18.1 to its neighbour. Every seat then
    got the tightest pair's answer — 4.8 units, three cards — including the
    seats with room to spare.

    Two changes, and they only work together. `sides_on` now walks the ring
    and places sides at equal *distances* (a 256-step polyline; the perimeter
    of an ellipse has no closed form), and each side's angle is the ellipse's
    inward normal rather than the parameter angle, so a mat is square to the
    table it is drawn on. And `side_half_widths` answers per side, from a
    separating-axis bound against the neighbours that side actually has,
    instead of one number for the whole table.

    Measured at the duel HUD's aspect: four seats 10.0 → 12.0, five 7.9 →
    12.0, six 4.8 → 9.9, eight 2.0–2.7 → 6.4.
    `no_two_seats_play_on_the_same_table` is the guard: a separating-axis test
    over every pair of footprints at every seat count, four aspects and with
    an opponent focused, which the old width rule — a bound on the distance
    between two *centres*, which says nothing about two rectangles turned to
    face different seats — failed from three seats up on a portrait canvas.

    The floor is the one place mats may still meet: a board is never narrower
    than one card, and a table crowded past that overlaps rather than drawing
    a mat a card does not fit on. The answer there is to seat fewer players.

30. **The bound was measured along the wrong line, and the boards paid for
    it twice.** *Fixed.* Two rectangles miss each other as soon as *some*
    line separates them, and for two rectangles four candidates suffice —
    each one's lane axis and each one's depth axis. What entry 29 left behind
    tried exactly one line, the one joining the two middles, which is sound
    and far too careful: at four seats it held every board to 10.0 when the
    near board could have been 13.7 without coming within a unit and a half
    of the seat on its left, whose mat lies *across* the table and takes up
    `half_depth` of the width rather than its own. The ring then grew to buy
    back width that was already there — 12.70 × 6.05 where 10.55 × 4.98 would
    have done, and four units of camera distance with it.

    With the right axes, four seats at the duel HUD's aspect: 10.0 units at
    23.7 of camera distance where it was 10.0 at 27.4. That headroom is what
    `MIN_POD_WIDTH` then spent, 10 → 12: eight cards a row for everybody up
    to five seats, 42% of the framed span against 38%, for two units of
    distance. Thirteen was measured too and refused — five seats is the table
    the camera has least room for, and it would have left it at 45.8 of a
    `MAX_DISTANCE` of 46.

    One board for the whole table, too. Widths had briefly been per side,
    each side taking what its own neighbours allowed; a table where one
    player's ground is wider than another's is a table where the wider ground
    is the one being played on. What is per seat now is only the focus, and
    it borrows from the other opponents rather than from the local seat,
    which used to drop from 10.0 to 8.5 at four seats every time a player
    looked at somebody else.

31. **Every card in the preview was drawn at half width.** *Fixed.* Entry 28's
    sibling: shift turning any card over meant the preview always builds both
    faces, and the two of them sat in the frame's flow as two items in a row
    one card wide — so taffy shrank each to half of it. `Visibility::Hidden`
    does not give a node's place back; only `Display::None` does, and a face
    that left the layout would resize the frame halfway through the turn. Both
    faces are absolutely positioned now, one on top of the other.

32. **A two-headed table came out twice the shape it asked for.** *Fixed.*
    The ring is sized so the whole span is the shape of the canvas — that is
    what `ring_for` is — and it measured the span as one board per side. A
    side of two reaches twice as far along itself, so the table was twice as
    wide as it asked to be: the camera fitted it by width, all four boards
    sat in the top half of the window, and the bottom half was bare felt.
    The busiest side's party divides the width now, and the team test asserts
    the span's shape as well as the seating.

33. **The shot framed the box around the table, not the table.** *Fixed.*
    Seats sit on a ring, so the corners of the rectangle around them are bare
    felt — at three seats the two that matter are a good four units outside
    anything anybody plays on — and the camera fitted those. It filled 86% of
    the width it was given and 81% of the height, binding on neither, which
    is a camera that could have come in and did not. Every card at the table
    was drawn smaller for felt nobody uses.

    `TableLayout::corners` is `extent` at the other tightness, and the
    horizontal fit asks each corner about its own depth — a mat at the near
    edge needs more room than the same mat across the table. Every pair of
    corners gives one division and the widest wins, so it is still arithmetic
    rather than a search. Measured at the duel HUD's aspect: three seats
    30.7 → 27.8 units of camera distance, five 44.1 → 41.1, eight 41.3 →
    39.2, and the width now fills 95–97%.

    And the table is **centred** in the band on both axes. The far edge used
    to be pinned under the tab strip with every spare unit opening up in
    front of the local seat: on a duel that was a fifth of the window of bare
    felt below the mats and the whole table riding high. A table too big for
    `MAX_DISTANCE` still pins the far edge, because a mat behind the tab strip
    is one nobody can see and a mat under the hand bar is one the player can
    pull into view.

34. **The table was framed to the last pixel, and looked cropped.** *Fixed.*
    Entry 33 made the fit exact and then spent all of it: `AIR` was 0.6 units,
    the shot pressed the outermost mats against the band on every side, and a
    photograph of it reads as a picture somebody cropped too tightly however
    correct the arithmetic is. Worse, 0.6 barely clears `ZONE_MARGIN` — a
    seat reports the box its *cards* stand in and its mat is drawn 0.55 wider
    than that, so the printed border of the near mat had 0.05 units of slack
    against the hand bar and nothing else did.

    `AIR` is 2.0 now: the mat's border, then felt. That is also about where
    `GLOW_SPREAD` fades out, so the halo under the seat being waited on stays
    in frame with the mat it belongs to. Five seats go from 41.5 to 44.2 units
    of camera distance, which is why `MAX_DISTANCE` went 46 → 64 — a limit
    sitting just above the furthest table does not stop that shot, it silently
    crops it, and it was capping how far a player could pull back besides.
    `a_seats_printed_border_is_inside_the_band_too` is the new assertion, over
    three window shapes; the tightness test now measures the framed hull
    rather than the bare table and is bounded on both sides, because "could
    have come in" and "pushed out until the table is a coaster" are different
    failures and only one of them was ever checked.

35. **A permanent's preview opened in the middle of the screen, and the stack
    had none at all.** *Fixed.* The hover preview was written for the hand,
    where a card has a place in the HUD's own layout and the bubble points at
    it. Everything else — a permanent on the felt, the top card of a pile, a
    card in the command zone — anchored at `None`, which meant the centre of
    the window: a panel three hundred pixels wide opening a foot away from the
    card it describes, over the middle of the table, on every hover. The stack
    panel was worse. It draws its cards an inch across, which is enough to
    recognise a spell and nowhere near enough to read one, and the whole panel
    was `Pickable::IGNORE` — so the one place where "what is about to happen,
    and to what" has to be read in a hurry could not be read at all.

    `Duel::hovered_at` carries the pointer position out of the `Over` event
    that set the hover, and `PreviewAt` says which of the three placements a
    card gets: the hand keeps its bubble and its caret, the keyboard cursor
    (which names a card without standing anywhere) keeps the middle, and
    everything the pointer found stands beside the pointer. `preview_place` is
    the arithmetic — beside, never under, flipping to whichever side it fits,
    clear of the tab strip, the hand bar and the phase rail — and it is a pure
    function with four tests, because the alternative is reading it off a
    photograph. Stack entries and their targets now carry `HandCardVisual`,
    which is what `pointer_hover` looks for.
