//! Ashnod's Battle Gear — {2} — Artifact
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {2}, {T}: Target creature you control gets +2/-2 for as long as this artifact remains tapped.
//! Set: 4ED #296 — Fourth Edition | Scryfall ID: 0bc11285-0891-4cc3-a056-b698911166c7 | Oracle ID: b5a390fd-2864-4481-84b4-41e8fac91a80
// PARTIAL — the untap-step choice is built; the pump is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASHNOD_S_BATTLE_GEAR,
    oracle_id = "b5a390fd-2864-4481-84b4-41e8fac91a80",
    scryfall_id = "0bc11285-0891-4cc3-a056-b698911166c7",
    faces = &[face!(
        name = "Ashnod's Battle Gear",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no Duration says \"for as long as this artifact remains tapped\": \
         WhileSourceOnBattlefield would keep the +2/-2 after the artifact \
         untaps"
    ),
    // NOT SUPPORTED: "{2}, {T}: Target creature you control gets +2/-2 for as
    // long as this artifact remains tapped." — `Effect::PumpTarget` can say
    // the effect (`Amount::Fixed(2)` and `Amount::Negated(&Amount::Fixed(2))`)
    // and `Filter::YOUR_CREATURE` the target, but its duration is
    // `Duration::WhileSourceTapped`, which does not exist; the nearest,
    // `WhileSourceOnBattlefield`, would leave the -2 toughness on a creature
    // after the Gear untaps, so the ability comes off the card.
    abilities = &[static_ability!(Filter::This, Modifier::MayChooseNotToUntap)],
);
