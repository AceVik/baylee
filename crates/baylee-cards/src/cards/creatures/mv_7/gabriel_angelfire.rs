//! Gabriel Angelfire — {3}{G}{G}{W}{W} — Legendary Creature — Angel
//! Oracle: At the beginning of your upkeep, choose flying, first strike, trample, or rampage 3. Gabriel Angelfire gains that ability until your next upkeep. (Whenever a creature with rampage 3 becomes blocked, it gets +3/+3 until end of turn for each creature blocking it beyond the first.)
//! Set: ME3 #148 — Masters Edition III | Scryfall ID: 2e349074-1402-44cf-be19-33a661cff3b6 | Oracle ID: e8521fdf-0896-4747-8c66-ea3bb69eb876
// PARTIAL — the upkeep keyword choice is off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "At the beginning of your upkeep, choose flying, first
// strike, trample, or rampage 3. Gabriel Angelfire gains that ability until
// your next upkeep." — the trigger (`Trigger::StepBegin` at upkeep) and the
// first three choices (`ModalTriggered` granting `KeywordSet::FLYING`,
// `FIRST_STRIKE`, `TRAMPLE`) are sayable, but no `Duration` ends at the
// controller's next upkeep (`UntilYourNextTurn` expires at the start of that
// turn, before its untap step, and `UntilYourNextUntapStep` is a different
// boundary), and rampage 3 has no keyword bit or "becomes blocked" trigger to
// grant at all. A "choose one" with an arm missing is a different card, so the
// whole ability comes off.

card!(
    index = index::GABRIEL_ANGELFIRE,
    oracle_id = "e8521fdf-0896-4747-8c66-ea3bb69eb876",
    scryfall_id = "2e349074-1402-44cf-be19-33a661cff3b6",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "the grant lasts until your next upkeep, and Duration has no \
         such variant; rampage 3 also has no keyword bit or becomes-blocked \
         trigger to grant"
    ),
    faces = &[face!(
        name = "Gabriel Angelfire",
        mana_cost = mana!("{3}{G}{G}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(4),
        toughness = Some(4),
    ),],
);
