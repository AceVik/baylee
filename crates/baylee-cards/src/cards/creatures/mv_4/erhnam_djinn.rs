//! Erhnam Djinn — {3}{G} — Creature — Djinn
//! Oracle: At the beginning of your upkeep, target non-Wall creature an opponent controls gains forestwalk until your next upkeep. (It can't be blocked as long as defending player controls a Forest.)
//! Set: VMA #207 — Vintage Masters | Scryfall ID: a1b20fb7-90f3-442c-b105-dfcaf619348d | Oracle ID: d48a38c9-3dcd-4c18-8840-1b057ede3ff0
// PARTIAL — the upkeep forestwalk grant is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ERHNAM_DJINN,
    oracle_id = "d48a38c9-3dcd-4c18-8840-1b057ede3ff0",
    scryfall_id = "a1b20fb7-90f3-442c-b105-dfcaf619348d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Erhnam Djinn",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(4),
        toughness = Some(5),
    ),],
    coverage = Coverage::Partial(
        "the grant lasts until your next upkeep, and Duration has no \
         until-your-next-upkeep variant: UntilYourNextTurn expires at the \
         start of that turn and UntilYourNextUntapStep is spent only by an \
         untap step that happens"
    ),
    // NOT SUPPORTED: "At the beginning of your upkeep, target non-Wall
    // creature an opponent controls gains forestwalk until your next
    // upkeep." — the trigger (`Trigger::StepBegin` at upkeep), the target
    // (`Filter::ControlledByOpponent` and `Filter::Not` a Wall subtype) and
    // the grant (`Effect::PumpTarget` with `KeywordSet::FORESTWALK`) are
    // all sayable, but no `Duration` ends at the controller's next upkeep.
    // `UntilYourNextTurn` expires at the start of that turn, before the
    // untap step the grant should survive, and `UntilYourNextUntapStep` is
    // a different boundary: an untap step skipped by an effect never
    // removes it. So the trigger comes off.
);
