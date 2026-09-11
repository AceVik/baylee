# Observed faults — from live play, 2026-09-08

Reported by the project owner while playing, plus what measurement has and has
not confirmed. Nothing here is fixed. An entry is promoted out of this file
when it has a test that fails for the right reason.

The point of the file is that a fault reported from a real game is worth more
than one found by reading, and it is also the easiest kind to lose.

## Client

### 1. Card images not loading — FOUND AND FIXED

Reported: "Kartenbilder werden nicht geladen/angezeigt."

**The cause was the build, not the client.** Bevy's `WebAssetPlugin` registers
the `http` and `https` asset sources behind two separate cargo features
(`bevy_asset/src/io/web.rs`, `#[cfg(feature = "http")]`), and the workspace
enabled only `https`. The gateway's art mirror is `http://127.0.0.1:28766/art/…`
in development, so every picture a signed-in client asked for died at
`Asset Source 'AssetSourceId::Name(http)' does not exist` — 134 of them in one
launch. The card fell back to its constructed face and the library pile to a
grey slab, which is exactly what "the images are not loading" looks like.

What kept it hidden for a week is the asymmetry: `use_art_base` is only called
from the lobby's `/auth/config` callback (entry 2), so a client launched with a
`SeatTicket` never adopts the mirror, stays on the `https` CDN, and draws every
picture perfectly. The two paths disagreed, and the healthy one was the one
being measured. The measurement below is right and was taken on that path.

Fixed by enabling both features, with
`textures::a_card_picture_can_arrive_over_either_scheme` asserting the pair.
The same defect was **both** of the owner's first two reports: the card *backs*
come from the same base (`images::back_url`), which is why face-down piles were
grey. It was also the third — "I miss the glowing for playable cards" — because
the gold ring was still being drawn the whole time, around a dark rectangle on
a dark bar. With the art back, the two lands in a seven-card hand light up
again.

*The original entry, kept because the two dead hypotheses are still worth not
chasing:*

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
    clear of the tab strip, the phase rail and the hand bar — and it is a pure
    function with four tests, because the alternative is reading it off a
    photograph. Stack entries and their targets now carry `HandCardVisual`,
    which is what `pointer_hover` looks for.

36. **A mixed table halved its ring for the sake of one side.** *Fixed.*
    Entry 32 gave `ring_for` the size of the busiest side, because a side of
    two reaches twice as far along itself as a side of one and a two-headed
    table without that came out twice the shape it asked for. On a table where
    *every* side is a pair that is right. On a mixed one — `--teams 1,1,2` is
    a two-on-one, `1,1,2,2,0` is two pairs and a player on their own — it took
    the worst side and applied it to the whole ellipse. A two-on-one on a
    square canvas came out on a ring 2.5 by 11.2, a sliver, and handed every
    seat 9.9 units of board where the promise is 12; four seats as a pair and
    two singles came out on the same sliver and collapsed to the one-card
    floor, 2.0 units, where the mats then overlapped each other.

    It is the *average* side now — the whole table's demand spread over the
    sides it has. A uniform team table is unchanged, because there the average
    and the worst are the same number; the mixed tables go 9.9 → 12.0 and
    2.0 → 12.0, at no cost in camera distance. A side that asks for more than
    its share takes a longer stretch of its own side, which is what the
    compartments have always done. `a_mixed_table_still_hands_out_one_board`
    walks four seatings at two aspects and asks the thing the seating is for:
    one board, the same for everybody, partners sharing a side and nobody
    overlapping.

37. **The mouse drove the table like a map, and the camera could not tilt.**
    *Fixed.* Left-drag slid the table around and only the right button turned
    it, which is the wrong way round for a thing you are looking *at* rather
    than travelling over, and is not what a player arrives expecting from any
    other 3D scene. It is the orbit convention now: left-drag turns and tilts,
    right- or middle-drag moves, the wheel still zooms.

    Tilt did not exist at all. `CAMERA_LEAN` was a constant everywhere,
    including in the transform, so the one thing a player could not change
    about the shot was the one a photographer changes first. `CameraRig::lean`
    carries it, bounded at both ends and for two different reasons: a card is
    a slab with a wall around its edge and a contact shadow under it and reads
    as a decal from straight overhead (`MIN_LEAN`, about 10° off plan), and
    there is nothing drawn behind the table for a flat camera to find
    (`MAX_LEAN`, about 55°). `CameraRig::home` still solves at `CAMERA_LEAN`
    and says so — a tilted shot is one the player has taken over, and
    `frame_table` has stopped writing to the rig by then.

38. **A three-player free-for-all sat down as a 2v1.** *Fixed.* The ring is
    shaped to the canvas, and equal distances along a 12.0 × 5.7 ellipse put
    the two opponents at 150° and 210° — side by side across the top of the
    table with their inner corners nearly touching, which is exactly the
    silhouette two allies draw when they really are sharing a side. Three
    seats each playing for themselves read as a team game, and the only way to
    tell was the life totals.

    A free-for-all is offered a **circle** now, and takes it if it can still
    seat everybody at the standard board and the camera can afford to stand
    where it puts them (`ROUND_COST`, 1.3). At three seats that is 0°, 120°
    and 240°, boards still 12.0 wide, and 22.3 units of reach against 17.9 —
    every card about a quarter smaller, which is what a table that reads as a
    table is worth. At four the arrangement was already a diamond and a circle
    would cost four fifths; at five and six the circle runs past `MAX_RING_Y`
    before it has handed anybody a board. Those all keep the canvas shape, and
    `a_bigger_free_for_all_stays_shaped_to_the_canvas` checks they were
    refused for that reason rather than by accident, by walking the circles
    itself and pricing them.

    The channel poured between the boards follows the ring, so it came out of
    this too: a flat lens 29 units across with two tilted mats slicing into it
    read as a pit in the middle of the table, and on the round ring it is the
    hollow the three boards are set around.

39. **The player's own board was drawn a fifth wider than everybody else's.**
    *Fixed.* Every seat is laid out on exactly the same 12.0 units — there is
    a test — and at a three-player free-for-all they were drawn 450, 381 and
    378 pixels wide. A lean is paid for by the seat furthest from the camera
    and collected by the seat nearest it, and the nearest seat is always the
    player's own, so the one board a player compares every other board against
    was the odd one out. A format is not something to read off the life
    totals, and a table where your own half looks bigger than your opponents'
    is the same mistake entry 38 was.

    Two terms make it up and the lean drives both: a board turned away from
    the camera keeps `√(¼ + ¾cos²)` of its width whatever the distance, and
    the near seat stands `lean · cos · radius` closer to the eye than the ring
    does while the far ones stand half that further away. The second shrinks
    as the lens lengthens, so the fix is both halves at once — `CAMERA_LEAN`
    0.40 → 0.24 and `FOV` 0.7 → 0.42, the same table framed the same way from
    twice as far off through half the angle. 18.9% → 6.3%, and it is the
    player's own board that comes back to the size of the opponents' rather
    than theirs that grow. `MIN_DISTANCE` and `MAX_DISTANCE` are distances
    through that same lens and moved with it.

    `every_seat_is_drawn_a_board_of_the_same_width` is the bound, and it is a
    bound rather than an equality: the last few per cent is the foreshortening
    above, and the only lean that spends it is zero — a table of decals.

40. **"I never draw lands."** *Measured, and the shuffle was not the
    culprit.* `GameRng::shuffle` is a textbook Fisher–Yates over a seeded
    ChaCha8 stream with Lemire's unbiased `below`, and the seed is 64 bits of
    OS randomness per game (`auth::new_game_seed` for a table, `fresh_seed`
    for an offline duel) — but the only thing asserted about any of it was
    that it is *deterministic*, which a broken shuffle is too. Both of the
    classic wrong loops (`1..len` instead of `(1..len).rev()`, `below(len)`
    instead of `below(i + 1)`) pass that test and leave the deck biased.

    `a_shuffle_puts_every_card_everywhere` is the property that was missing:
    60 000 deals of a 60-card deck, chi-square over where the top card lands,
    bounded on both sides — under 120 because a right algorithm passes it and
    the two wrong ones fail by thousands, over 20 because a stream too even
    for 60 000 random deals is not a random stream either.

    What was actually wrong was the **deck**. `Allytifact` — the deck
    `dev-table` and the client's offline duel both deal — held 31 lands in 99
    cards, 31.3%, where a hundred-card singleton deck normally runs 36–38%.
    That is 2.19 lands in an opening seven, and a landless hand one game in
    fourteen, however well it is shuffled. Nine basics in and eight of the
    deck's eleven clone effects out to the sideboard leaves 39 lands in 99
    cards, 39.4%, which is where a five-colour deck with this much fixing
    belongs; the opening seven now holds 2.76.
    `an_opening_hand_holds_what_the_deck_holds` writes the arithmetic down so
    the next person to feel unlucky can tell the two causes apart.

    The cards leave by the **sideboard** rather than by deletion, because this
    file is also the coverage proof — "both decks fully implemented" is its
    first line — and a card cut out of it stops being named anywhere. Eight
    clones is the one cut that costs no *mechanic*: Phyrexian Metamorph, Helm
    of the Host, Storm of Saruman and Reflections of Littjara stay, so every
    copy path in the pool is still dealt.

    One real limit came out of it: the gateway caps **every** card at four
    copies, basic lands included, and `the_starter_deck_is_one_the_gateway_
    will_accept` enforces the same. Deck-construction rules exempt basics, so
    a deck that wants twelve Islands cannot be saved today. Not fixed here —
    the basics added stop at four.

41. **The question you must answer was the smallest thing on screen.** *Fixed.*
    The prompt bar sat in the bottom-right corner in 88%-black at 13 px — the
    corner furthest from the two things a player is already looking at, their
    own board and the hand under it — and the zone browser was a five-card
    panel pinned to the top-left, over the seat tabs.

    Both are **sheets** now: `tabletop::parchment`, generated the way the felt
    is, centred where the thing they describe is. The prompt slip stands over
    the near edge of the player's own board, so the question and the cards
    that answer it are one place to look; the browser is a sheet laid in the
    middle of the felt, eight cards across, which is the gesture of putting a
    stack of cards down on a table. Felt is the ground, parchment is a sheet
    you read, brass draws the lines and gold belongs to the local seat — one
    material per job, which is what makes a panel say what kind of thing it
    is before a word on it is read.

    Two things came with it. A sheet is **opaque**, where the panels it
    replaces were 88% black over the table — a question read through whatever
    card happened to lie under it. And a browser row now **previews on
    hover**: the tray draws cards 74 px across, enough to pick one out and
    nowhere near enough to read one, so a library search was a wall of
    thumbnails to be recognised by picture alone. `preview_anchor` answers any
    object the view can resolve, which covers a graveyard card that is not the
    top one, an exile pile, and the cards the engine is *showing* this seat.

42. **The table was a plank with a river in it.** *Replaced.* Two complaints
    in one sentence — "the table is not nice, try a rough material and more of
    a casino-mat green", and "give it some height, not just a flat 1 px plane"
    — and the second is the one that says what was really wrong: the slab was
    a `Rectangle`, a mathematical plane with a picture on it, so no camera
    angle in the range the player can dial found an edge to it.

    It is a **slab** now, in the same sense a card is: `rounded_slab_mesh`
    builds both, a rounded top face with a wall around its edge, and the table
    passes `(0.0, -TABLE_THICKNESS)` where a card passes
    `(CARD_THICKNESS, 0.0)` — the body hangs below the plane everything else
    on the stage is placed against, which is the half that would have broken
    the board silently. Nothing lights this stage, so the wall reads as a wall
    only because `APRON` is a *darker colour* than the rail above it. That is
    an assertion, not a taste: `the_baize_is_a_casino_green` fails if it stops
    being true.

    The surface is casino baize inside a padded leather rail, and the epoxy
    river is gone with the timber it ran through. What the river carried is
    kept: the phase lamp — `phase_light` graded by the step, energy to the
    fourth so combat blooms while a main phase stays a quiet line — runs round
    the **rail** now, entering at the active seat's own edge. The rail is the
    only surface at this table no card is ever laid on, which is the property
    the resin channel was chosen for in the first place.

    And the slab does not reach the edges of the window. The camera frames the
    layout plus `AIR` and the slab is cut to the layout plus `SLAB_MARGIN`, so
    there is a band of something-else all the way round, which is what entry
    43 needed. `the_table_stops_before_the_window_does` measures exactly that,
    against the window and not against itself.

    **The corner and the rail were then both too much**, and the owner said
    so: "the table should have less border corner and the table border is way
    too thick". `table_corner` was `span.min_element() * 0.42` — a racetrack,
    chosen back when the *corners* were the only place a sky could show
    through, and made pointless the moment `AIR` opened a band round the whole
    slab. It is `* 0.16` now, which is a table's corner rather than an oval's
    end. `RAIL_WIDTH` went 2.0 → 1.6 → 0.9 over the same two passes: the rail
    is padded leather round a playing surface, and at 1.6 it was reading as
    the frame of a painting. `a_table_has_a_corner_and_not_a_chamfer` bounds
    the corner from both sides — under 0.08 of the short span it is a bevel,
    over 0.28 it is a racetrack again — and checks that the arc still sags
    clear of its own chord by more than the rail is wide, because a rail that
    follows a straight line round the bend is a chamfer whatever the radius
    says.

43. **"Day and night, with clouds and a sun, then stars and a crescent moon —
    both with animated shaders and glow."** *Built, and it needed the table to
    move over first.* There was nowhere to draw it: the slab was cut wider
    than the camera's frame, so the felt reached every edge of the window and
    a backdrop would have been a layer nobody could ever see. Entry 42's
    racetrack gives up the corners, and `AIR` went from 2.0 to 3.5 so there is
    a band all the way round as well. That costs about nine per cent of a
    card's drawn width, and it is the whole price of the feature.

    The sky is **one quad parented to the camera**, painted in screen space.
    Not a `Skybox` (that wants a cubemap), not a UI node (UI draws over the 3D
    table), and emphatically not a plane in the world: this camera looks
    *down*, so a sky placed in the scene would be under the table. Screen
    space is also what lets the sun and the moon be put in the two upper
    corners, which is where the visible sky actually is.

    Day is a gradient with two drifting cloud decks — the upper one thinner
    and faster, so there is a depth to it — lit on the side the sun is on by
    sampling the same field a little way towards it. Night is stars as discs
    on a jittered grid, twinkling in *brightness* rather than size (a star
    that changes size crawls), with a crescent taken out of a disc by a second
    disc and a halo that belongs to the whole body. Dawn and dusk add a warm
    band low in the frame, scaled by `SkyPhase::glow` — its own number, not
    something derived from `day`, because a sky that reddened in proportion to
    how much night was in it would be reddest at midnight.

    Two things it deliberately is not. It is **not** Magic's day/night
    designation (CR 731): no card in the pool is daybound or nightbound,
    `baylee-view` carries no such state, and a client inventing one would put
    a rules claim on the table the engine never made. And the hour it follows
    is **UTC**, not the player's zone — a timezone database is a large
    dependency for one number and a browser will not hand one out — so a
    player at the ends of the world sees a sky up to half a day out, which is
    what `Day` and `Night` are for. Measured live, both ways: the sky strip
    moves 22/16/8 per channel over two seconds while the felt beside it moves
    0/1/1, and with `reduce_motion` on the two frames are byte-identical.

44. **"The table and the battlefield lines are unsharp."** *Drawn instead of
    stretched.* A seat's mat was a 512 × 256 image laid over a board about
    thirteen units wide, which at this camera is four texels to the physical
    pixel: every edge on it — the rim, the lane seams, the corners — was a
    soft ramp four or five pixels across, and no filtering setting fixes that,
    because a stretched image has no idea how large it is being drawn.

    `matmat.rs` and `shaders/mat.wgsl` draw it. A signed distance with
    `fwidth` gives an edge one pixel wide from any distance, and three of the
    owner's complaints turn out to be the same change:

    - **Sharpness.** The rim is now an antialiased edge rather than a
      resampled one.
    - **"The battlefield lines field should have less border radius."** The
      corner is `tabletop::MAT_CORNER`, a length in **table units**. It could
      not have been asked for before: `seat_mat` took a fraction of the
      shorter side of a texture, so the only way to know how round a mat came
      out was to work back through the image's size — 6% of 256 texels over a
      6.05-unit-deep board is about 0.37 units. It is 0.18 now.
    - **"Make the battlefield lines border glow for the current player."** A
      short comet travels the rim of the active seat's mat, once every 3.6
      seconds. A *travelling* light rather than a border that brightens and
      dims together, because a pulsing outline is the shape every interface
      uses for something is wrong, and this says only "it is your turn" — a
      thing that is true for most of a game and has to be able to sit there
      without nagging. It could not have been a texture at all without
      regenerating an image every frame.

    It is driven by `Mood::on_turn`, which had to be added: `Standing` is a
    *rank* and collapses the two questions, so a seat holding priority reads
    as `Priority` whether or not the turn is theirs. Brightness answers "who
    is everybody waiting for" and the rim light answers "whose turn is it",
    and on any turn where an opponent responds to something those have
    different answers — a light driven off the rank would leave the active
    seat and follow the response.

    `seat_mat` stays as the arithmetic's readable form and its test bench:
    every number both it and the shader use is a `tabletop::MAT_*` constant,
    and `the_shader_and_the_generator_agree_about_the_mat` fails if the two
    drift. One meaning did change with the move — the mood's brightness now
    scales the mat's **alpha** rather than a white tint, so a seat that has
    lost fades into the felt instead of drawing a dark rim over it, which is
    the reading `zone_brightness`'s own doc already claimed.

    Measured live: over one second the active seat's mat moves 84/70/63 per
    channel while the opponent's — same shader, same material, `on_turn` at
    zero — moves 3/2/2 and the bare felt moves 2/2/1.

45. **"Both need atmo lights on table — night moon blue, day sunny."** *Done
    as a tint, not a lamp.* The stage carries no light source at all and may
    not: scene lighting on card art would make colour identity unreadable,
    which is the one thing this table is not allowed to do. So
    `sky::table_light` hands the felt shader a multiplier on the table's own
    colour, and `under_sky` applies it to the cloth, the rail and the apron
    and to nothing else. A multiply rather than a mix towards a colour,
    because a warm sky over green baize has to be able to lift the red end
    without touching the green — a mix drags every channel towards the light's
    hue and turns the cloth grey at both ends of the day. The phase lamp is
    exempt: it is light the *table* emits and carries a meaning, and a wash
    that went blue after sunset would be saying something untrue about the
    turn.

    Measured, both ways, on the same patch of bare felt: `(23.8, 65.8, 43.3)`
    by day against `(19.6, 59.9, 45.1)` by night — warmer and brighter under a
    sun, cooler and darker under a moon.

    **The switch between them is one movement, not two.** `sync_sky` eases the
    phase and both the sky and the table read the eased number on the same
    frame, so nothing has to be told a transition is happening. `FADE_RATE`
    went from 2.5 to 0.55, which turns a second-and-a-half smear into about
    six seconds: caught at startup with the sky pinned to `Night`, the sky's
    blue channel walks 113 → 84 → 69 → 63 → 59 → 58 while the felt walks with
    it, 22.2 → 19.6 in the red. And `DRIFT` went from 0.0065 to 0.025, because
    a cloud deck chosen so that a player reading a card never catches it
    moving had succeeded completely — the sky now moves 8/5/2 per pixel over
    three seconds while the felt beside it moves 0/0/0.

    That measurement is also what found the fade's one wrong audience.
    `hang_sky` seeded the sky at full day and left `sync_sky` to ease it
    towards the truth, so the six seconds were being spent on *every launch
    after dark* — a player opening the game at eleven at night watched a
    sunset nobody had asked for, which reads as a picture correcting itself
    rather than as weather. The sky is hung at the hour it is now, and the
    rate is what it was chosen for: a change the player makes.

    Measured at 22:00 UTC, screenshotting from the moment the table first
    draws: the sky reads `(14.5, 15.6, 29.5)` on the earliest frame there is
    one — frame 31, half a second after launch — and has not moved a tenth of
    a channel a hundred frames later. The counter-test is the same patch with
    `sky` pinned to `day`: `(100.1, 152.9, 208.9)`, seven times the blue.
    That is what the first half-second used to be.

46. **"Es gibt noch dieses ausklappbare 2D battlefield — das soll komplett
    weg."** *Removed.* `OwnBoardOverlay` drew the local seat's battlefield a
    second time, flat, in the same half of the screen the camera already
    frames that board in. Two drawings of one board is two hover states, two
    glow registers and two click paths to keep in step — and this one kept a
    power nothing else on screen has, which is covering the game: it is the
    panel entry 42's black screen turned out to be. Gone with it: the knob,
    the slide animation, the `X` action, `Duel::overlay_open`/`overlay_t` and
    the redraw-gate field. `hud/overlay.rs` keeps its name and its job, the
    retained HUD tree.

    Removing an action is what made `Keymap`'s reader tolerant, and that is
    the part worth keeping. `Keymap` is `#[serde(transparent)]` over a map
    keyed by `Action`, so a stored blob still naming `toggle-overlay` was
    refused **whole** — and `Preferences::from_json` answers a refusal with
    the defaults, so one retired row would have silently reset every key
    every player had ever bound. An unknown name is dropped and the rest of
    the map kept now, which is the same bargain `#[serde(default)]` already
    makes for every other field, and it holds in both directions of an
    upgrade. `the_shipped_keymap_is_still_recognised` reads a literal blob
    that names `toggle-overlay`, so the day the reader stops being tolerant
    is the day that test goes red.

47. **"Packe die vertikale Phasenleiste horizontal nach oben unter die
    players Leiste."** *Done, and it changed more than the position.* A turn
    is a **sequence**; a column of twelve buttons made the eye read it as a
    list of settings. Laid out left to right under the seats — two rows,
    opponents' turns above your own, the turn number at the left where the
    eye starts — the rail is the shape of the thing it describes, and the
    step the game is in travels along it. Every button is the same width
    (`flex_grow`, `flex_basis: 0`), because they are twelve equal parts of
    one turn and a rail that sized them by their labels would be claiming
    the draw step is smaller than declare-attackers.

    Four of the request's five clauses are drawn rather than said:

    - **The elevation shadow.** `hud::elevation_shadow(down)` casts it, and
      takes a direction because the hand bar wants the mirror of it — one
      function rather than two constants, since both strips stand at the same
      height over the same felt and hand-written pairs drift the first time
      either is tuned.
    - **The transition.** The HUD tree is rebuilt whenever anything in
      `HudRevision` changes, and a step change is one of those — so the
      button that is current was *born* current and the one before it no
      longer exists. `PhaseNow.lit` therefore starts at zero on a fresh
      entity and is eased to one by `light_the_current_step`, which makes a
      value that only ever climbs the whole of the animation. It writes the
      border and the light and leaves the background alone, because `Feel`
      owns that one and two systems writing one component is a fight the
      schedule decides. The system is *run* in a test, not merely declared.
    - **Hover, focus and press.** `ambience::Feel` already did this for every
      button in the lobby, driven by `PickingInteraction`, and the duel had
      none of it. A live rail button now carries one; the keyboard focus
      keeps its accent border.
    - **The dead phases.** This is the one that had to go in the *model*.
      `RailRow::grants_priority` is false for untap and nothing else — "No
      player receives priority during the untap step, so no spells can be
      cast or resolve and no abilities can be activated or resolve" (CR
      502.4) — and `PhaseOrders::toggle` refuses it, `is_skipped` answers
      true whatever the stored table says, and `move_selection` steps over
      it. A rail that only *drew* it unclickable would still turn it green
      under a preset, a keyboard, or a blob stored by a client that had the
      button.

    **Cleanup is deliberately not dead**, which is a narrowing of what was
    asked for and worth saying out loud. Priority there is rare rather than
    impossible: an ability that triggers during the cleanup step gives the
    active player priority and they may cast spells (CR 514.3, 514.3a).
    Declining a window the rules grant and declining one nobody can use are
    different things, and only the second is the client's to decide on a
    player's behalf. Cleanup is red by default instead, which is a preference
    and can be changed. That closes the second half of entry 10.

    The two arrow buttons at the rail's foot are gone, and the fast-forward
    came back where the request said it should: `PromptAction::SkipTurn`
    stands beside Pass on the prompt slip. "Pass this window" and "pass every
    window until my next turn" are the same decision at two sizes, and a
    player who has just been offered the first should not have to look in a
    corner for the second.

    Everything anchored to the tab strip moved down with it — the zone chips,
    the browser's centring frame, the stack panel, the preview's band, and
    `Canvas::hud`, which is the one that matters: the camera frames the table
    against what the HUD covers, so a rail the framing did not know about
    would be a mat drawn underneath it. `Canvas.right` is zero now and stays
    as a field, because the next panel to take a side needs somewhere to say
    so.

48. **The table: more angle, a narrower edge, and that edge turned inside
    out.** *Done.* Four requests in one paragraph, and they pull in different
    directions, so the trades are written down rather than split between a
    constant and a screenshot.

    - **"Neige den Kamera Winkel noch etwas mehr."** `CAMERA_LEAN` 0.27 →
      0.36, and this one costs something. A lean is paid for by the seat
      furthest from the camera and collected by the nearest, which is always
      the player's own; the widest board on an eight-seat ring goes from 6.3%
      to 10.6% wider than the narrowest, and
      `every_seat_is_drawn_a_board_of_the_same_width`'s bound moved 1.08 →
      1.12 to allow exactly that. It is the third time this angle has been
      asked for after being told what it trades against, which is a decision;
      the bound is the measurement plus a hair, and it still fails the shot it
      was written for (18.9%). The lens was tried as a way out and buys
      nothing: 0.34 at a `FOV` of 0.36 spreads the same 8.2% as 0.33 at 0.42,
      because the larger of the two error terms is foreshortening and no lens
      shortens that.
    - **"Border schmal und den Border Radius noch etwas weniger."**
      `RAIL_WIDTH` 0.9 → 0.55, `table_corner` 0.16 → 0.11 of the short side.
      The sag check in `a_table_has_a_corner_and_not_a_chamfer` still holds:
      an 0.84 sag across a 0.55 rail.
    - **"Drehe es um ... wie ein Altar."** The felt fell into `FELT_DEEP`
      over 1.8 units as it reached the rail and the rail crowned in its own
      middle — between them, a surface sunk inside a frame, which is a tray.
      An altar is the other way round: the top is the highest thing there is
      and its edge is rounded over and away. So the cloth now keeps a crest of
      light *at* its boundary (`ROLL`, `ROLL_LIGHT`) and the rail falls from
      `RAIL_LIP` at the inner edge to `RAIL_HIDE` at the outer one on a
      quarter circle's cosine rather than a ramp — a linear fall reads as a
      chamfer cut at forty-five degrees.
    - **"Der Tisch braucht vielleicht ein Spotlight."** `under_lamp`: an
      elliptical pool that **darkens the ends** rather than lifting the
      middle. That direction is forced — the cloth is already as bright as
      `the_felt_is_dark_enough_to_read_cards_against` allows from both sides,
      so a lamp that lifted the centre would be a table competing with its
      own cards. Elliptical because the slab is much wider than it is deep and
      a round pool would light the near and far seats while leaving the two at
      the ends in the dark. A multiply on the table's own colour, like
      `under_sky`, and for the same reason: there is no light in this scene
      and there cannot be one.

    **"Die Battlefield Linien rand animation ist zu schnell, sie soll wie ein
    Atemfluss wirken und auch glühen."** The rim light was a short comet at
    3.6 seconds a lap. It is three terms now: `TURN_BASE`, the light the whole
    rim of the active seat carries all the time — which is the difference
    between travelling and *glowing*; a swell that goes round once every 11
    seconds covering more than half the rim at a time; and a breath the rim
    takes together every 7. Two periods with no common multiple worth
    noticing, so the rim never repeats a pose, which is what makes slow
    movement read as alive rather than as a loop. `TURN_REACH` 0.45 → 0.85
    spreads it across three times the width of border, which is the other half
    of "glühen": the same light in one pixel is an outline.

    Measured live over eight seconds, on the same table: the active seat's rim
    swings `4.31 / 3.64 / 2.90` per channel, while the opponent's rim — same
    shader, same material, `on_turn` at zero — moves `0.00 / 0.00 / 0.00` and
    so does the bare felt between them.

49. **The prompt slip: a strange background, an unreadable aside, and buttons
    that huddled in the middle.** *Fixed.* Six requests in one sentence, and
    the first of them was a real defect rather than a taste.

    **"Es sieht (der Hintergrund) noch seltsam aus."** `sheet()` was inserted
    on the slip *itself*, and bevy paints a node's `ImageNode` over its
    **content box**. The slip has `padding: axes(22, 13)`, so the parchment
    was drawn in the middle with a ring of flat `palette::PARCHMENT` around
    it — twenty-two pixels wide at the sides, thirteen top and bottom — and
    the sheet's own rounded corners cut *inside* the slip's. Two concentric
    rounded rectangles in two colours, which is exactly what "strange" looks
    like when nobody has a word for it. `hud::sheet_surface` is the fix: the
    parchment is an absolutely-positioned first child, which is measured
    against its parent's *padding* box — precisely the missing ring — and is
    `Pickable::IGNORE` so a surface never takes a click. The zone browser had
    the same defect with a sixteen-pixel ring and is fixed with it.

    Measured rather than argued: eleven samples across the old padding band
    now span ten levels of grain (224…214), where a flat fill gives one value
    at every x.

    **Text.** Italic, a faint warm `TextShadow`, a little of the sheet through
    the ink (`SLIP_INK` at α 0.94, `SLIP_SOFT` at 0.92), and bracketed asides
    in a grey that has had the warmth drained out of it — an aside is a
    different *kind* of sentence, a key to press or a count the board already
    shows, and grey says so where another shade of brown would only say
    "further away". The split is `client_core::prose::bracketed`, in the model
    with a test, and an **unclosed** bracket greys nothing: one stray
    character must not drain the rest of a line.

    Italic needed a second font file. `Inter.ttf` is variable on `opsz` and
    `wght` and nothing else, and `TextFont` carries a face and a size — no
    style, no synthetic oblique — so a slant this client cannot ask for is a
    slant it has to ship. `Inter-Italic[opsz,wght].ttf`, the same family under
    the same OFL entry `NOTICE` already names.

    **Buttons.** The row spans the sheet and every answer takes an equal part
    of it (`flex_grow: 1`, `flex_basis: 0` — grow alone divides only the slack
    left after the labels, so three answers with three different words still
    come out three different widths). Each carries `soft_shadow()` and the
    `ambience::Feel` every other button in the client has. The unlead answers
    are filled in `SLIP_GHOST` rather than `Color::NONE`, because a drop
    shadow under a surface that is not there renders as a dark rounded hole.

50. **A button went dead under the word it is named after.** *Fixed, and it
    was never the slip's bug alone.* `Feel` animates whatever the pointer is
    over, and a `Text` is a `Node`: a label inside a button is a pickable
    child sitting in front of it, so the hover stopped at the letters. The
    lobby's `button` and `chip` had always marked their labels
    `Pickable::IGNORE`; the phase rail and the prompt slip had not, so both
    lit up in their padding and went dead across the middle.

    Found by measurement and it would not have been found any other way — the
    first diff was over the *word* and moved 0/0/0, which reads exactly like
    "the animation was never wired". Hovering the same button's padding moved
    155/156/148. With the labels ignored, the pointer on the word moves
    140/141/134 and the neighbouring answer 2/0/1.

51. **Every mat was drawn at the size the table had on its first frame.**
    *Fixed.* A seat's zone is geometry — a mesh cut to the mat's width, a
    glow quad under it, pile places at fixed points beside it — and none of
    it is a uniform that can be rewritten, so `sync_zones` only ever built it
    once. The layout is legitimately rebuilt more than once, though: the
    first one is laid out against a *guessed* canvas aspect
    (`canvas_aspect.unwrap_or(16.0 / 9.0)`, because the canvas is not known
    until the HUD has been laid out), and focusing a seat widens it and
    shrinks the rest. So the cards moved to the new layout and the mats,
    glows and pile places stayed at the old one.

    Traced rather than guessed, by printing both positions from the two
    systems that draw them: the local seat's `half_extent.x` was **9.864**
    when its zone was built and **12.841** when its commander was placed —
    the commander stood two and a half card widths outside the mat it belongs
    to, level with a pile place that was drawn somewhere else entirely. It
    had been invisible because the only things that ever sat on a pile in a
    fresh duel were the wells themselves, which were wrong *together*.

    `Zone` carries the `SeatSlot` it was built for now, and a seat that has
    moved or resized is torn down and built again.

52. **The zones beside a mat: five slots, a mark in each, and a deck that
    has a thickness.** *Done.*

    - The command zone is drawn as one place per commander. `PileKind`
      gains `Command2`, matched to the second commander by `ObjectId` —
      which survives the moves that make a card a new object (CR 400.7), so
      a partner that dies, goes home and comes back down lands on the slot
      it left. A seat with one commander is drawn one slot and a seat with
      none is drawn no command zone at all; nothing reflows around a hidden
      one, because commanders are fixed before the first turn (CR 903.3) and
      a player should learn once where their graveyard is.
    - Exile moved up beside the graveyard, so the right-hand column reads
      draw → die → gone with distance from the hand. It is level with the
      creature row, which the old doc forbade: that row is where attackers
      step *forward*, along the seat's `away`, while a pile stands
      `PILE_REACH` out **sideways** on bare table. The guard was against a
      pile touching the row, and at 1.45 units clear it does not.
    - Every well carries its zone's mark — three leaves, a headstone, a
      barred ring, a crown — as arithmetic in the texture rather than a glyph
      out of a font, because the table is 3D and has no text on it
      (`docs/legal.md` §2). It is covered the moment a card lies on the pile,
      which is the whole design: the mark is the empty state and the card is
      the answer to it.
    - A pile is drawn as the deck it is: as tall as it has cards, capped at
      thirty, built from at most fourteen slabs so the block stays solid
      however many are in it, with a contact shadow that widens as it grows.
    - **And the top card of a graveyard is face up again.** The backing slabs
      used to be drawn *above* the card they belonged to, offset diagonally
      by a fifth of a card, so the last card into a graveyard was covered by
      a fan of card backs. They hang below it now, as children of the card so
      they follow every glide and are despawned with it — as loose entities
      they were never despawned at all, and every card that had ever lain on
      a graveyard left its slabs standing there for the rest of the game.
    - The hand bar's flat copy of the command zone is gone. It was the only
      zone that existed twice, in two sizes, in two renderers, with the 3D
      slot sitting empty underneath the copy.

53. **Summoning sickness was half a rule, measured on the wrong clock.**
    *Fixed.* CR 302.6 has two sentences and only the second one — "a
    creature can't attack" — was ever implemented. The first is about `{T}`
    and `{Q}` in an activation cost, and nothing checked it: a mana creature
    cast on turn three made mana on turn three, and the engine *offered* it,
    so this was not a client drawing an affordance that would be refused.
    The offer and the acceptance agreed with each other and both were wrong.

    Underneath it, `turn_start_timestamp` was one number for the whole game,
    stamped at the beginning of *every* turn. The rule says "continuously
    since **their** most recent turn began", so a creature cast on your turn
    is still summoning sick right through the opponent's — with one shared
    clock it woke as soon as anybody untapped. Combat could not see that,
    because attackers are declared on your own turn, where the two readings
    agree; it only became reachable once an activated ability was gated on
    the same predicate. The stamp lives on `Player` now, and the comparison
    against it is strict, because what it holds is the last stamp issued
    *before* the turn began rather than the first one issued during it.

    The third part is what the projection said. `combat::summoning_sick`
    tested haste and the clock and left the *type* to its callers, so the
    view reported it for every permanent that entered this turn — a land
    played this turn came back `true`. Both client readers masked it back
    off with a `CREATURE` test of their own, so nothing was drawn wrong, but
    the fact a client was handed was not the fact the rule is about. The
    type test is inside the predicate now, where both sentences of the rule
    put it, and a Vehicle answers to it the moment it is crewed because the
    type comes off the projected characteristics. The client keeps its mask:
    the shape of the view did not change, so no `VIEW_VERSION` bump refuses
    a host built before this, and one bit compare is what stops such a host
    putting a whole opening board to sleep.

    *Open, found on the way:* `CostPart::UntapSelf` does not require the
    permanent to be tapped, so `{Q}` can be paid by something already
    untapped. One line, but a different bug from this one.

    *And the drawing.* The half of the request that was not the rule: a
    summoning-sick creature was drawn with a uniform four-percent luminance
    breath, which is nothing on art whose own luminance already varies by
    forty points — a player could not tell a sleeping creature from one with
    dark art. It is a white balance and a blanket now: the face goes cold
    under a moon, a soft veil lies heavier at the foot of the card than at
    the head, and its upper hem rises and falls on a five-second breath —
    a moving *edge*, which is caught where a brightness pulse of the same
    size is not. Measured on a live duel: the sick permanent moves 7/6/3
    levels across half a breath while every other permanent on the table and
    the prompt slip are byte-identical, and its hover preview — the same
    arithmetic in the UI shader — moves 21/21/20.

54. **"Fetchländer erzwingen es nicht, dass das gefatchte Land getappt rein
    kommt. Bitte bei allen Fetchländern korrigieren."** *Fixed — in the other
    direction, and the report was right about the family being wrong.*

    A fetchland does not put its land in tapped. That is what the life
    payment buys: Polluted Delta costs {T}, 1 life and the land itself and
    puts an untapped dual on the table, while Evolving Wilds costs nothing
    but the land and puts a basic in tapped. Applying the report as written
    would have broken the six fetchlands that were right in order to match
    the four that were not.

    Because four of the ten were wrong, and in the direction that makes the
    report make sense. `marsh_flats`, `misty_rainforest`, `polluted_delta`
    and `scalding_tarn` searched with `Find::BATTLEFIELD_TAPPED`;
    `arid_mesa`, `bloodstained_mire`, `flooded_strand`, `verdant_catacombs`,
    `windswept_heath` and `wooded_foothills` searched with
    `Find::BATTLEFIELD`. The acceptance decks — and the decks in the
    gateway's store — hold both halves at once, so one game shows a
    fetchland tapping the land it finds and the next shows one that does not,
    which is exactly the observation. All four are `Find::BATTLEFIELD` now,
    and the pool dump moves four `tapped: true` to `tapped: false` and
    nothing else.

    Each of those four carried a header quoting the printed "put it onto the
    battlefield" *and* a comment saying "Untapped is the whole difference
    between a fetchland and Evolving Wilds; `Find::BATTLEFIELD` says so".
    Only the code disagreed, and nothing compared the two. The engine test
    was worse than absent: `fetchland_searches_island_or_swamp_tapped`
    asserted `Status::TAPPED` on the fetched Island, so the pool's own suite
    held the bug in place. It is
    `a_fetchland_puts_its_land_in_untapped` now and asserts the opposite.

    *The half of the report that is a real question, closed by measurement.*
    A search puts the card onto the battlefield itself rather than through
    the land-play path, so a fetched **tapland** honouring its own "enters
    tapped" is not something a card file can promise. It does:
    `apply_enter_modifiers` scans the journal for zone changes rather than
    for how a permanent arrived. `a_fetched_tapland_still_enters_tapped`
    fetches an Irrigated Farmland with a Polluted Delta and reads the status
    back, so that is now a test rather than a reading of the code.

    A **shockland** is the same question with a seam in it, and it is the one
    reading of the report that would still have been a live bug.
    `EnterModifier::Tapped` writes a status and returns; `TappedOrPayLife` is
    the only arm that publishes a `Pending` and returns *mid-scan*, so it has
    to survive being raised while a search is finishing resolving — `apply`
    publishes the priority pending and `apply_enter_modifiers` overrides it a
    line later. If that override lost, a Hallowed Fountain fetched with a
    Delta would arrive untapped with no question asked, which is precisely
    "das gefatchte Land kommt nicht getappt rein". It does not lose:
    `a_fetched_shockland_still_asks_the_question` drives the fetch, reads the
    `PayLifeOrEnterTapped` prompt back off `pending`, declines it, and finds
    the Fountain tapped and the life total untouched. Both halves, because a
    question that is asked and then ignored is the same bug one step later.

    *And the check that would have caught it.* `xtask validate` compares the
    header against the `CardDef` — it just had nothing to say about this
    word. It does now: every `finds:` list is read against the printed
    "onto the battlefield tapped", and only a *put* counts, so a land whose
    own text taps it as it enters is not mistaken for one that taps what it
    finds. Re-introducing the bug in one card fails the command by name.

55. **"Entferne die aktuelle Zonendarstellung und ersetze sie komplett durch
    die neue. Repariere das schließen Icon … Mach diesen Dialog resizeable und
    verschiebbar."** *Done, in four parts.*

    *The old display was a second renderer for three zones.* A strip of pile
    chips sat above the board drawing the local seat's graveyard, exile and
    command zone as counts that opened the browser. Those three piles now
    stand on the felt with their real top card lying on them (entry 52), a
    pile's top card is a `Placement` like any other, and `input.rs`'s
    `open_pile` already turned a tap on one into `Browser::open_at`. So the
    strip was deleted and the pile *is* the button — which is what the chips
    themselves said the mat corners should have been all along, and could not
    be at the time only because the counts they replaced were `TextSpan`s
    inside one text entity with no layout node to click.

    `open_pile` was wired and nothing had ever clicked it, which is the shape
    of the `client-input-gap` lesson, so the deletion needed a witness rather
    than a reading. `a_tap_on_a_pile_opens_it` goes through `activate_card` —
    the function the pointer calls — and fails by name when that last branch
    is removed; `a_tap_on_a_library_opens_nothing` holds CR 401.2. Live, a tap
    on the opponent's command-zone card opens the sheet on
    `Command · House AI (1)`.

    *One thing the chips did that the felt cannot.* Cards in `looking_at` are
    *shown* to a seat without being asked about — a reveal, a turned-over top
    card — and they live in no zone the table draws. `Browser::follow` only
    runs when a **choice** arrives, so before this the "Zones" chip was the
    only way to see them. `Browser::saw_reveal` opens the sheet on them,
    edge-triggered on the *ids*: per-frame would make the panel impossible to
    close, and a length comparison would merge "one reveal ends, another
    begins" into no opening at all.

    *The close icon.* Measured in the live shot it was a 26.5 × 21.5 pill
    with a 6.5 px cross adrift inside it, `Color::NONE` behind it, and no
    `Feel` — a stray mark on the sheet rather than a control. It is 24 × 24
    now with a 13 px glyph, a `SLIP_GHOST` fill (not `NONE`: a fill-less node
    under a drop shadow renders as a hole) and a `Feel`, which the tabs, the
    sort key, the direction arrow and the filter box also gained. They were
    the only buttons in the client that did not answer the pointer.

    *Bigger, shadowed, translucent, with grey brackets — which already
    existed.* All four were built for the prompt slip in milestone 5, and a
    second treatment of the same parchment is how two halves of one interface
    start disagreeing. `slip_line` split into `slip_text(.., italic)`: the
    slant is the slip's own voice (a question being asked), everything else
    belongs to the sheet. The browser passes `false` and gets the rest.
    Sizes went 13 → 16 for the title and 10/11 → 12 for the controls, and the
    bracketed run the treatment greys is new — every zone tab now says how
    many cards are in it, which is the one thing the deleted chips carried
    that nothing else did. `BRASS` also came off the sheet as *text*: it is
    1.9:1 on parchment, which is why the current tab read fainter than the
    ones beside it.

    *Draggable, resizable, remembered.* `browser::Placement` is a rectangle
    inside the band between the phase rail and the hand bar, in logical
    pixels — the grid inside is cards at a fixed size, so a sheet that scaled
    with the window would show a different number of columns on every screen.
    It lives in `baylee-client-core` because all of it is arithmetic, and
    arithmetic with a window in front of it is arithmetic nobody tests.
    `fit` shrinks before it moves and never writes itself back. The geometry
    is in `ClientSettings` (the owner said *clientseitig*) and not
    `Preferences`, which travels with the account.

    Two mechanics worth not re-deriving. The drag is a `Pointer<Press>` plus a
    per-frame cursor read rather than `Pointer<Drag>`, because the duel HUD is
    a retained tree rebuilt on every snapshot and hover — a chain bound to the
    header entity dies when that entity is despawned mid-gesture. And the
    geometry stays out of `HudRevision`, or a rebuild would run per pixel.
    `HudRevision` *did* gain the window size, which nothing in it followed
    before: a HUD built for one size stayed that way through a resize, latent
    everywhere the overlay reads `windows`.

    *And a defect the test found in itself.* `/pointer` presses and releases
    in one call, so a drag cannot be proved through `dev-control`;
    `input::dragging` proves it headlessly on the sheet's `Node`. Its release
    then wrote the developer's real `~/.config/baylee/client-settings.json`,
    moving the sheet in their own client by the delta the test had invented.
    `ClientSettings::save` is a no-op under `cfg(test)` now — the in-memory
    write is what a test has business asserting, and the encoding is proved by
    `settings_round_trip_through_json` without touching a file.

56. **"Gehe noch Mal über die UI und schau, dass sie gleichmäßig und
    ordentlich aussieht."** *Done. Four defects, and every one of them was
    something the eye reads before it reads any single panel.*

    *Four left edges down one side of one screen.* The tab bar held its tabs
    8 logical pixels from the window's edge, the phase rail under it held its
    steps 10, and the mana chip and the prompt slip stood 12. Vertically the
    chip cleared the hand bar by 10 and the slip beside it by 12. Each was
    defensible alone; together they read as carelessness, because a shared
    margin is a decision and four near-misses are an accident. `hud::EDGE`
    and `hud::ABOVE_HAND` are the two numbers now, and every one of those
    panels measures from them.

    *A state-dependent border moved the row it was in.* `bevy_ui` **adds** a
    border to an auto-sized node's box, so the seat tab's rim — one pixel at
    rest, two when active or focused — made the tab whose turn it was two
    pixels taller and wider than its neighbours, and the whole strip shifted
    sideways every time the turn passed. The rim is two pixels always and
    says what it has to say in colour: `ACTIVE` for the turn, the seat's own
    identity colour for the focus, that colour at 40% alpha at rest. Which
    also gave `is_focused` a channel of its own — it had been carried by that
    width and by nothing else.

    *The tab bar did not state its height, so a tab that grew grew under the
    rail.* Measured on the running client: the active tab's gold bottom
    border read (72, 55, 31) where its top read (214, 163, 79), the same gold
    seen through the rail's 88% black. `hand::TAB_H` states it now — 56, with
    no slack in it: 2 + 2 of border, 4 + 4 of padding and two line boxes of
    16.8 and 13.2 with 2 between them come to 44, and the strip's own 6 above
    and below make exactly 56. After: (203, 154, 74) against (214, 163, 79),
    the difference being the tab's own shadow.

    *Which the pass then walked straight into.* Fixing the caret's column
    meant reading the tab's height off the screen, and the arithmetic that
    reproduced it said there was no room in the strip at all — so anything
    that adds a **third** line to a tab overflows it silently. Exactly one
    thing does: the commander-damage track (CR 903.10a), which is drawn only
    once a commander has connected, so no ordinary game shows it. Forced on
    and measured: the tab stood 58.5 tall, its top border cut off by the
    window's edge and its bottom border drawn over the phase rail. It sits
    beside the life total now, where it costs width instead — the cheaper of
    the two on a strip that states its height and not its width, and where a
    second life total belongs anyway. Not free either: the bar neither wraps
    nor clips, so eight seats all carrying a track widen every tab by about
    ninety pixels. `nothing_new_is_stacked_into_a_seat_tab` counts the calls
    that stack a row into a tab and expects two; the layout that would prove
    it directly exists only inside a running renderer.

    *And every node of the track is `Pickable::IGNORE` now,* which on the
    life line is load-bearing and not tidiness: anything pickable in front of
    a control stops `PickingInteraction` at itself, so `Feel` would have gone
    dead across the whole track — the label finding a third time, invisible
    in an ordinary game for exactly the reason the overflow was.

    *And the one the pass was not looking for.* `HudRevision` never compared
    `chosen_index`, so the answer chooser's brass highlight followed the
    pointer — which rebuilds the tree — and not the keyboard, which does not.
    It is a field now, and `every_field_of_the_revision_is_both_compared_and_
    assigned` reads `HudRevision`'s field names out of `hud.rs` and checks
    each against both halves of `overlay.rs`, so the next field cannot be
    forgotten the same way.

    *Two smaller rules that came out of it.* `Feel` owns `BackgroundColor`
    from `base` every frame, so a fill it does not know about survives one
    frame — every control that changes fill with state needs that state in
    the revision. And the fill under a control that is *off* is
    `palette::SLIP_GHOST`, never `Color::NONE`: on a node carrying a drop
    shadow that is not "no fill" but a hole with the shadow showing through.

    *The trade taken knowingly.* The priority caret is drawn whatever
    happens and merely goes `Color::NONE`, because a caret that appeared and
    vanished shoved every seat beside it. Inline that indented the name by
    the caret's own advance and left the counts under it hanging seventeen
    pixels to its left — two lines of one tab that no longer shared an edge.
    It stands in its own column now, which costs nothing vertically (the
    marker is shorter than the stack beside it) and fixes the edge.

57. **"Implementiere den Tag/Nacht Zyklus mit Mechanik in die Engine.
    Implementiere auch einige dazugehörige Karten."** *Done — and on the way
    it found a 9/7 Demon that anybody could cast for {0}.*

    The designation is CR 731: one game-level `Option<DayNight>` that starts
    as neither and, once set, never returns to neither. The check is the
    untap step's *second* turn-based action (CR 502.2), and it reads the turn
    that just ended — day becomes night when that player cast no spells,
    night becomes day when they cast two or more, one spell holds either way
    in both directions. That reading is why `GameState::previous_turn` had to
    exist: `PerTurn::reset()` runs at the top of `begin_turn` and the active
    player swaps in the same breath, so the top of `begin_turn` is the single
    instant at which both the seat that just played and its spell count are
    still true. `the_check_reads_the_turn_that_just_ended_and_not_this_one`
    is what fails if that snapshot moves a line.

    *Daybound and nightbound had to become per-face keywords first.* CR
    702.145g says "no permanents with daybound", and 702.145c fires on a
    front face; a card-level keyword makes the first unsatisfiable and the
    second fire on a permanent that has already transformed. `FaceDef` owns
    `keywords` now, with face 0 inheriting the card's when it states none —
    so all 1360 other cards keep writing one line. The delta that a pool dump
    would have shown is one card: **Twining Twins** put its flying and
    vigilance at card level, and its Adventure back, Swift Spiral, is an
    Instant. It had been casting a flying, vigilant instant. The test that
    was written to catch the family caught it instead, and is narrowed to
    what it can actually assert —
    `daybound_and_nightbound_are_printed_on_the_face_that_has_them`.

    *The free spell.* `cast_wizard.rs` offers every non-front face as a cast
    *mode*, skipping only lands and `!castable_from_hand` — and a transformed
    back prints no mana cost, so it was offered for `{0}`. Nothing in a
    `CardDef` says which layout a card was printed in (CR 712.2 vs. 712.4a),
    so `castable_from_hand` is the whole difference, and every card in the
    pool had left it at the default. It had never fired only because every
    transforming DFC there had a *land* back. Three did not: Westvale
    Abbey's Ormendahl, Profane Prince (9/7), Hostile Hostel's Creeping Inn
    (3/7) and Balamb Garden, Airborne. All three are stubs, so the fix is in
    the generator rather than in the files — `stubgen::render_face` writes
    the refusal for any back face that prints no cost and is not a land,
    which is the rule that tells the two layouts apart: an MDFC's back, a
    disturb back and an adventure all print one.
    `a_back_face_with_no_printed_cost_is_never_castable_from_the_hand` reads
    it back out of the compiled pool. The first version of that test read
    *nightbound* instead, which would have guarded the five new werewolves
    and let the next Delver of Secrets straight through.

    *Two layout defects, both found by photographing the running client and
    neither by reading the code.* The rail's head now says which turn it is
    and what the game is, and the designation block was first laid out as a
    column: measured 30.5 logical tall beside the turn number's 19.5, on a
    strip where the two sit side by side. Both are Rows of a stated
    `HEAD_H = 22.0` now, the turn block carrying a transparent 1px border so
    it is measured the same way, and both read logical 73.0–92.5 after.
    Then night: the pill was `PANEL` on a `PANEL` rail and measured (13, 15,
    21) against (12, 14, 20) — a pill by day and a bare floating glyph by
    night. Both fills are `PANEL_LIT`; what changes with the designation is
    the sun in `PARCHMENT` against the moon in `INK`.

58. **"Machine-owned; fix the reader" — and the three lands that had been
    milling the wrong player.** `codegen` had only ever written a file that
    was missing or still carried `// GENERATED STUB`, so a card one of the
    readers wrote *in full* was hand-owned from the moment it existed. That
    is backwards for the thing it was protecting: `landgen` and `forgegen`
    each wrote hundreds of cards, so a rule that read a sentence wrongly got
    every card printing that sentence wrong — and the file in front of you is
    one of them. Patching it leaves the other ninety-nine broken and teaches
    the reader nothing.

    A card a reader wrote is machine-owned now: it carries
    `// IMPLEMENTED — generated by xtask codegen` and `codegen` rewrites
    it on every run, so the only edit that survives is the one in the rule,
    which corrects every card that rule reached at once. `xtask adopt --name`
    is the deliberate way out, for a card that genuinely needs something the
    reader cannot say; it strips the marker and the file is a person's from
    then on. `validate` reports the split, 212 hand-owned against 364
    machine-owned, because that ratio is what says how much of the pool is a
    testable rule rather than a transcription.

    *What the first run found.* 62 of the 364 machine-owned files had drifted
    from what their reader produces today, and the drift had been invisible
    for exactly as long as the files were untouchable. Fifty-nine were a
    stale summary on the marker line. Three were rules: **Duskmantle, House
    of Shadow**, **Nephalia Drownyard** and **Orzhova, the Church of Deals**
    each print "target player mills…" / "target player loses 1 life", each
    offered `TargetSpec::AnyPlayer` — and each then applied the effect to
    `PlayerRel::You`. Three lands that asked which opponent to mill and
    milled their controller instead. The reader had already been corrected:
    `forgegen` wrote all three in `6bd876d`, and nine commits later the same
    day `457647f` taught it that an effect handed a target says
    `PlayerRel::Chosen` rather than `You`. By then the three files existed,
    which under the old rule made them hand-owned, so the correction reached
    every card written after it and none written before.

    The lesson is the ownership rule itself, and it is the reverse of the
    usual instinct: a generated file that cannot be regenerated is not
    protected, it is *stranded* — cut off from every fix its generator will
    ever receive, with nothing reporting that it has fallen behind.

59. **"Das Flackern der Karten und anderer Sachen auf dem Feld bleibt" — the
    report had a second cause, and it moved nothing.** *Fixed.* Entry 35 and
    `db252ca` found a real defect behind the same sentence (a pickable preview
    panel opening onto the pointer that opened it) and closed the report with
    it. The owner played again and the flickering was still there.

    The second cause was `card_ui.wgsl`. Every card in the hand, the preview,
    the stack and the tray carried a metallic coat driven by `globals.time` on
    a continuous six-second loop, so the whole hand bar changed a little on
    every frame with nothing hovered, nothing moving and nobody's turn. What
    named it was a 24-by-16 grid of mean absolute pixel difference over two
    frames of a still table: 18–30 per cell across the hand bar, 0 across the
    felt. A repositioning bug moves cards, and the cards had not moved — which
    is why "I think it has something to do with the rerendering, repositioning
    of cards" pointed at the wrong half of the client, and why reading the
    glide and the hover for a second session would not have found it either.

    `crates/baylee-client/src/sheen.rs` is the fix and it is also what the
    owner asked for on its own merits: the sweep is one-shot, bottom-right to
    top-left, on a card that is *new* — drawn, played, or opened in the
    preview — and quicker for each card in a burst, retiring itself when the
    band is over. The material stays metallic; only the motion is spent. The
    same grid on a still table now reads a maximum of 2.4 and 0–2 across the
    hand, the residue being the amber activatable border the owner asked to
    keep.

    Two lessons, both about the measurement rather than the shader. A fix
    measured against the fault it *found* still has to be measured against the
    sentence that was **reported**; entry 35's grid was never taken, so the
    hand's own churn sat in plain sight through two sessions and was once
    written off as the ambient sky. And a `/screenshot` round trip costs about
    2.4 s in a debug build, so a sub-second animation cannot be photographed at
    its own speed at all: the sweep had to be slowed to twelve seconds to be
    seen, and the first attempt to read it diffed two *adjacent* frames that
    had landed on the same phase and reported 2.8 where the widest pair of the
    same run reported 78.7.

60. **A mana pool was never emptied, in any step, ever.** *Fixed.*
    `ManaPool::empty_at_step_end` was written for CR 106.4, carried the rule
    in its own doc comment, and had exactly one caller in the workspace: its
    own unit test. Nothing in the engine called it. Mana therefore floated
    across steps, phases and whole turns — a Forest tapped in the first main
    phase was still paying for things in the opponent's end step three turns
    later — which is a whole category of illegal play rather than a card
    behaving oddly.

    It was found by measuring, not by reading. A test about which face of an
    adventure is offered kept being answered by the *previous* turn's mana,
    and the print that proved it was `pool=ManaPool { plain: [0, 2, 0, 0, 0,
    0] }` at the start of turn 3 with nothing tapped in it. The grep came
    after: one caller, and it was the unit test.

    The fix is one loop at the top of `advance_step`, which is the only place
    a step or phase ever changes. It is before the match rather than after,
    because the turn-based actions in those arms belong to the step being
    *entered* — the draw of CR 504.1 happens in the draw step, and mana made
    in the upkeep must not pay for anything after it.

    The blast radius is the interesting part, and every bit of it was a test
    that had quietly been living on banked mana:

    - Eleven engine tests drove a turn with a loop that tapped every land the
      moment one was offered. The first offer is in the *upkeep*, so they now
      spent the whole board there and reached the main phase with nothing.
      All of them tap in a main phase now.
    - `mdfc_tests` gave Sheoldred four swamps for a `{3}{B}{B}` creature. It
      only ever worked because two turns of four swamps is eight mana.
    - A miracle (CR 702.94) was offered at the instant of the draw, which is
      before anybody holds priority, so with an empty pool it was a question
      nobody could ever say yes to. The reveal is a triggered ability and the
      cast happens when it resolves, so the offer now comes after the
      priority round — the only place this engine lets a player float mana,
      because a cost is paid from the pool and CR 601.2g is compressed away.
    - The client's `duel_flow` suite stopped reaching `ChooseCards`,
      `ChooseTargets` and `YesNo` at all. Its greedy loop guarded tapping to
      its own main phase and then reached the same lands through
      `LegalActions::abilities`, where a mana ability is also listed: it
      tapped out in the upkeep every turn. Three questions of coverage were
      resting on mana no rule allows to still be there.

    The lesson is the one about dead code that is not dead: a method whose
    only caller is its own unit test is a rule the codebase *describes* and
    does not *apply*, and it reads as implemented from every direction —
    the name, the doc comment, the passing test. Grep for callers, not for
    the rule.

## Fourth pass, 2026-09-10 — found while making a card-less ability say so

### 30. Ward never fires — FIXED, and the cause written here was wrong

`AbilityDef::Ward { mana }` is read by `trigger.rs` and the whole path
exists: three synthetic effect lists, a scan that looks for a `SpellCast` or
`AbilityTriggered` event, and a `PlayerMayPayOr` that counters the spell if
the tax is declined. It fires for nothing. Measured on Twining Twins
(ward {1}), targeted by an opponent's Path to Exile: the Path resolves with
no question asked, and the same is true of a token copy of it.

**The timing diagnosis that stood here was wrong**, and it is worth keeping
the correction visible: it read that the scan's window opens before the
spell has targets, because `SpellCast` is journalled when the cast begins.
`cast_wizard` writes `obj.targets` and records `SpellCast` *after* it
(`cast_wizard.rs` ~977, then ~1046), so the scan sees exactly what it is
looking for. A probe in `trigger.rs` on the failing case printed the window
entry `SpellCast { object: ObjectId(123#0), player: PlayerId(1) }` with
`targets_this=true`, and then the `AbilityTriggered` the scan queues. The
trigger is collected, goes on the stack above the spell, and is resolved.

It dies in resolution, one line lower down. `Effect::PlayerMayPayOr` asked
`eval::players` which seat owes the tax; ward says `PlayerRel::
ControllerOfTarget` — the caster — and `eval::players` answered `vec![]` for
it, because that relation and `Chosen` need the resolution's own context and
only `resolve::players_of` has it. The `.first().copied()?` then returned out
of the effect entirely: no question, and not even the
`CounterTargetSpellOrAbility` fallback that a declined tax runs.
`players_of`'s own doc comment already named the trap — it is how Abraded
Bluffs shipped as a land that deals no damage — and **ten** sites were
reaching past it.

**The same seam, four more victims**, found by sweeping the pool for both
relations rather than for ward:

- **Path to Exile** — `OptionalBasicLandSearchFor { ControllerOfTarget }`.
  Entry 7 above read this card's ramp half off its source and said it
  implements exactly what it prints. It does, and it did nothing.
- **Bojuka Bog** — `ExileGraveyard { Chosen }`: a land that costs a land drop
  and exiles no graveyard.
- **Jace, the Mind Sculptor −12** — `ExileLibraryAndShuffleHand { Chosen }`.
- **Ertai Resurrected**'s two halves — `DrawCardsFor { ControllerOfTarget }`,
  which fault 34 hides: a `ModalTriggered` is never collected at all, so the
  draws are not reachable to be wrong yet.

Fixed by routing all ten sites through `players_of`, and by changing
`eval::players` to return `Option<Vec<PlayerId>>` — `None` for the two
context relations. The `Option` is the half that matters. `vec![]` reads
exactly like "no seats matched", so a new caller swallows the trap without
noticing; a `None` cannot be consumed without a visible decision, which is
the same convention-into-a-build-failure idiom the card tests use. Two
callers legitimately have no resolution to ask — `eval::target_options`
enumerates legality *before* one exists — and now say so in a sentence
instead of by accident.

Three tests, one per victim the pool can reach today:
`keyword_tests::ward_taxes_the_opponent_who_targeted_it`,
`card_tests::path_to_exile_offers_the_ramp_to_the_creatures_controller` and
`card_tests::bojuka_bog_exiles_only_the_graveyard_it_targeted`. Each asserts
the **seat** and not merely that a question was asked, because offering
Path's search to the caster would be just as wrong and would pass a test
that only counted a `ChooseCards`. Each was mutated by reverting its own
site alone: each failed, and only that one.

Prowess is the same branch and is *not* the same fault — it reads the
`SpellCast` event alone and needs nothing from the spell's targets. It is
also reachable, and is now played rather than read: **Sokka, Tenacious
Tactician** prints menace and prowess, gives both to every other Ally his
controller has, and makes an Ally token on each noncreature spell, so one
board proves the card-less half of the synthetic-trigger fix by playing
it — `token_tests::a_token_grows_on_the_prowess_it_was_lent`, where the
first Brainstorm makes the token and the second is the spell its borrowed
prowess answers. With the old guard back in place the token stays 1/1.

(The first draft of this entry said the pool had no implemented prowess
creature. It was grepped for the printed word `Prowess` under `cards/`
rather than for `KeywordSet::PROWESS`, which is how a keyword is spelled
where it counts — the pool-wide-claim lesson, once more.)

The fault that found this one is still worth recording as a pair: the change
that surfaced it made the card-less half of an ability handle honest, which
is what let a copy of a warded creature be *queued* a trigger at all. That it
then fired for nobody was a second fault underneath the first, and it turned
out to be the larger of the two.

## Fifth pass, 2026-09-10 — from the owner's UX report

### 31. Suspend was offered before it could be paid for — FIXED

Reported as *"Suspend works different. The text says: Rather then cast this
spell from your hand, PAY x and exile …, so first tap and pay mana, then
suspend."* — which is exactly what the card says and exactly what the engine
half of it did not ask.

`abilities.rs`'s hand-zone scan offers cycling and suspend. Every activation
branch there goes through `can_afford` before it pushes; the suspend branch
was `AbilityDef::Suspend { .. } if sorcery_timing`, matching on the turn and
on nothing else. `actions.rs` **does** charge the cost — `mana_pay::pay`,
answering `cannot pay the suspend cost` when the pool is short — so the offer
and the answer disagreed: a card with suspend was listed as suspendable off
an empty pool and refused on the click. Suspend's first ability is an
activated one with a cost (CR 702.62a), so it belongs to the same rule as its
siblings, and `can_afford`'s mana half is now `can_pay_mana`, called by both.
`offer_tests::a_suspend_card_is_offered_only_once_its_cost_is_on_the_table`
holds it; with the old guard back the first assertion fails.

The report's parenthesis — *"(There are other suspension costs not only
mana)"* — is true of Magic and is **not fixed**, because it is a DSL
limitation rather than a defect in this pool. `AbilityDef::Suspend { counters:
u8, cost: ManaCost }` (`baylee-cards-dsl/src/ability.rs`) can say only mana,
and the pool prints exactly two suspend cards — Ancestral Vision
(`sorceries/mv_0/ancestral_vision.rs`, Suspend 4—`{U}`) and Profane Tutor
(`.../profane_tutor.rs`, Suspend 2—`{1}{B}`) — both of them mana-only. So
nothing in the pool as it stands is misrepresented. A suspend cost with a
`CostPart` in it wants `cost: Cost` here and `pay_cost` on the apply path,
which is the change to make when a card that needs it arrives.

### 32. A card with no mana cost was castable for nothing — FIXED

Found while wiring entry 31's client half, on the same card. Ancestral Vision
prints **no** mana cost, which CR 202.1a says is not the same thing as a cost
of `{0}`: a card with no mana cost cannot be cast unless something gives it an
alternative cost or lets it be cast without paying one. Every affordability
probe in `casting.rs` asks only whether the pool covers the cost, and a pool
covers a blank cost trivially — so a card whose entire text is a suspend
ability and "target player draws three cards" sat in `legal.castable` from the
hand of anybody who reached their own main phase, for free, and the suspend
ability was the *worse* of the two things to click.

`ManaCost` had the distinction all along and nothing read it: Ornithopter's
`{0}` is one `Generic(0)` symbol, a blank is no symbols at all, and
`to_string` has always written them as `"{0}"` and `""`.
`casting::has_a_printed_cost` is that question, asked by `can_cast` and by
`cast_wizard::cast_options` — the two probes that must agree — and asked of
the **printed** cost, because `with_less_generic` rebuilds a cost symbol by
symbol and does not write back a `Generic(0)`, so `{0}` reduced by nothing
comes out blank. That trap cost one debugging pass: guarding `normal_cost`
instead of `c.mana_cost` made Mox Opal uncastable.

The pool half is the same class as entry 31's DSL note and was live rather
than theoretical: `face!`'s default cost is the *blank* one, and **Mox Opal**
and **Pact of Negation** both print `{0}` and both omitted the field, so both
were sitting in the blank set. They now say `mana_cost: mana!("{0}")`.
Codegen has always emitted the distinction correctly — these two are
hand-written files. `offer_tests::a_face_with_no_mana_cost_has_another_way_
out_of_the_hand` is the guard: a face may print no cost only if the card
suspends or the face has an alternative cost, so the next hand-written `{0}`
card that forgets the field fails the build instead of becoming quietly
uncastable.

The client mirrors the rule in `reachable` for the reason it mirrors the
timing one: a blank cost is payable by an empty pool, so a suspend-only card
was lit indigo with a plan of no taps at all, and the click armed a run that
finished at once and asked the engine to cast a card it will never offer.

Noted and not fixed: `progress.rs` cites CR 702.61 for the suspend countdown
and `actions.rs` cites CR 702.62 for the suspend cost. One of the two is
wrong; neither was verified here.

### 33. The self-play harness was calling finished games loops — FIXED

Found because an engine change (entry 32, which lands next) tripped the
acceptance soak's floor: two of the four games no longer finished. The change
was correct and the floor was right to fire — but the games it was firing on
were not looping.

`play_report`'s own repeat detector keys on `snapshot_hash` plus turn, step
and the open question, and halted on the **second** sighting of a key. One
repetition is not a loop. A question that interposes itself between the last
pass of a priority round and the end of the step reopens that round with the
game in exactly the state it was in when the round began, and `snapshot_hash`
covers the rules state and not the engine's queue of offers waiting to be
made — so the two rounds hash alike. A **declined miracle** is that shape, and
it is the commonest one there is.

Measured, not argued: an instrumented replay of seed 42 printed the same hash
at actions 110–114 and the step advancing at 115, in a game that is won on
turn 7. Seed 7 was the same and is won on turn 56 — it had been halting at
turn 13 since before this pass, which is why the floor stood at three of four
rather than four.

The detector now halts on the third sighting, which costs a genuine loop a few
more samples and nothing else, and the floor is four of four: a floor under
the measured value is exactly what let two games sit unexamined.

## Sixth pass, 2026-09-10 — found while building a deck to photograph a copy

### 34. A modal triggered ability never triggers — CONFIRMED, cause found

Found by accident. A scratch deck for the copy-of-a-token proof opened with
Aether Channeler, whose ETB is "choose one — Bird token / bounce a nonland
permanent / draw a card". It was cast, it resolved, and **nothing happened**:
no token, no card drawn, no permanent returned, no mode asked, an empty stack,
and `last_error` reporting no refusal. Every channel a fault normally arrives
through was silent, which is what makes it worth an entry — a player reads
that as the card doing nothing, and there is nothing to report.

The cause is in `crates/baylee-engine/src/trigger.rs`, and it is the shape the
`activated-conditional-is-a-forgotten-twin` lesson describes. `collect` is the
only place a `PendingTrigger` is made, and both of its loops open with

```rust
let AbilityDef::Triggered { trigger, once_per_turn, .. } = ability else {
    continue;
};
```

so an `AbilityDef::ModalTriggered` is skipped in silence. It never becomes a
pending trigger, never reaches the stack, and is therefore never asked about.
The far half of the path is already built — `progress.rs` reads
`ModalTriggered` in three places to offer the modes and to resolve the chosen
one — so what is missing is the collection arm, not the mechanism.

Five cards in the pool carry one, and all five are marked `Implemented`:
Aether Channeler, Charming Prince, Ertai Resurrected, Inspirit Flagship Vessel
and Primaris Eliminator. That is the part to fix first — a card the
deckbuilder offers as playable whose printed ability cannot fire is worse than
a stub, because a stub says so.

Not investigated, and both wanting a rule looked up rather than recalled:
whether the mode is chosen at the moment the trigger is put on the stack (a
spell chooses its modes as it is cast, and a triggered ability is not cast, so
the two are separate sentences in the rules), and whether `trigger_count` and
`once_per_turn` need anything the modal variant does not already carry.

## Seventh pass, 2026-09-11 — from the owner's play session

### 35. A triggered ability fired once for a whole batch of events — FIXED

Reported from a live game: Aang and Katara made six Ally tokens, which was
right, and then "four of them disappeared and only one of the two was handled
correctly — haste and 2× +1/+1; the other should have got it too."

Two different things, and only one of them is a rules defect.

The tokens did not disappear. Five of the six were identical 1/1 Allies and
the client draws identical permanents as one pile (`CardGroup`), so a board of
six read as two. That is a legibility problem in the client — the pile does
not say how many it holds — and it is on the UX backlog, not here.

The rules defect is why one token differed from the other five. Wartime
Protestors says "whenever another Ally you control enters, put a +1/+1 counter
on that creature and it gains haste until end of turn", and the ability fired
**once** for six simultaneous arrivals. `trigger::collect_for_objects` scanned
the new journal entries for each ability and then did this:

```rust
for entry in events {
    if matches(trigger, &entry.event, state, permanent, obj.controller) {
        …push `times` triggers…
        break; // one trigger per event per ability — next event
    }
}
```

The comment describes the intended rule correctly and the `break` implements a
different one: it leaves the *event* loop, so the ability fires at most once
per collection window however many matching events that window holds. The
first Ally to arrive was answered and the other five entered unnoticed.

Panharmonicon was not involved and was not wrong: `trigger_count` doubled the
one trigger that did fire, which is exactly the 2× +1/+1 that was reported.

Four copies of the same `break` were there — the emblem scan, prowess, granted
triggered abilities and the ordinary one. All four are gone.

**What made the old shape look defensible**, and the check that had to come
before removing it: an ability worded "whenever **one or more** … " does fire
once for a whole batch. Storm the Vault is the only card in the pool whose
printed text is worded that way, it is an unimplemented stub, and `forgegen`'s
trigger table reads exactly four Forge modes (`ChangesZone`, `Phase`,
`Attacks`, `Taps`) and refuses the batch ones outright, so no machine-owned
card encodes batch semantics either. Nothing was relying on the accident. When
Storm the Vault is written it wants a `Trigger` variant of its own, not this
line back.

Also fixed by the same change, and worth naming because neither had ever been
seen to be wrong: Ondu Cleric's rally gained life once for a batch of Allies
instead of once per Ally, and prowess counted one spell per window rather than
every noncreature spell cast in it.

The test is `a_rally_trigger_fires_once_for_every_ally_that_entered`, and its
assertion is a **count** in both directions: each of the three tokens carries
exactly one +1/+1 counter. Zero fails the old code; anything above one would
fail the other way this loop can be written wrong, which is firing per
permanent *and* per event and giving N² triggers for N tokens.

### 36. A permanent that changed zones kept what it remembered — FIXED

Reported in the same session: "flicker a tapped creature makes it enter
tapped", and then "I flickered a land and it came back tapped as well".

CR 400.7 makes an object that changes zones a new object with no memory of its
previous existence, and `GameState::move_object` — the one door every zone
change goes through — cleared the layer cache and the copiable base and
nothing else. The tapped bit, the marked damage, the deathtouch flag, the
counters and what the permanent was attached to all rode into the exile zone
and back out again.

So this is not a flicker fault. Aminatou's −1 and Ephemerate were how it was
noticed; a creature bounced and recast, and anything reanimated out of a
graveyard, had the same hole.

The reset is in `move_object`, conditioned on `from_zone == Zone::Battlefield`
rather than on the arrival. That direction is load-bearing: several effects
write these fields *before* the move, and `SearchDest::Battlefield` is the
clearest — it taps the land it fetched and then moves it onto the battlefield,
so a reset on the way in would undo the half of "put it onto the battlefield
tapped" that does the work. "Enters tapped" as a printed characteristic is
applied by `apply_enter_modifiers` afterwards and is untouched either way.

`riders` deliberately stay: a card exiled from the battlefield is linked to
whatever exiled it (Skyclave Apparition reads its own exiled card's mana value
from one), which is among the exceptions CR 400.7 is written around. So do the
spell-shaped fields — `x_value`, `kicked`, `targets` — which a permanent
resolving off the stack still needs and which no permanent ever writes.

`sba::put_into_graveyard` had been clearing damage and the deathtouch flag by
hand, which was one caller doing what every permanent leaving the battlefield
needs. Those two lines are gone, so the two cannot drift.

Before clearing `counters` and `attached_to` there: every dies- and
leaves-the-battlefield trigger in the pool was read, because `matches` and
every dies-effect read the object *after* the move and there is no LKI
snapshot. All four (Solemn Simulacrum, Soulherder, Skyclave Apparition,
Reveillark) read the graveyard, the exile rider or nothing at all — none reads
the dead object's counters, tapped-ness, attachment or projected power. A card
that did would be an LKI fault of its own, not a reason to leave this.

The test is `a_blinked_permanent_comes_back_untapped`: a land tapped the only
way a player can tap it — by using it — then Aminatou's −1, then the land is
untapped.

### 37. A blinked permanent is the same object — RECORDED

`Effect::Blink` moves the permanent to exile and back **under its own
`ObjectId`**, so everything that names an object by id survives a flicker. The
`version` field is documented as "bumped on every zone change (CR 400.7)" and
`move_object` does bump it, but nothing in the engine ever compares it — it
reaches only the snapshot hash.

What that predicts, none of it checked yet: a removal spell targeting a
creature still resolves on it after the creature is blinked in response, where
it should be countered for having no legal target; an `EffectFilter::ObjectIs`
pump written for the old permanent still applies to the new one; and an
Equipment stays attached across the blink (fault 36 clears the blinked
permanent's own `attached_to`, which is the other side of the same wire).

The fix shape is a new object rather than a moved one — the same thing
`create_token_copies` already does with a set of copiable values — or, much
cheaper, a `version` comparison at the two or three places that resolve a
stored `ObjectId` back into a target. One test proves it either way: Path to
Exile on Llanowar Elves, blink in response, the Elves are still on the
battlefield and Path was countered.
