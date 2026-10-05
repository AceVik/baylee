//! Clergy of the Holy Nimbus — {W} — Creature — Human Cleric
//! Oracle: If this creature would be destroyed, regenerate it.
//! Oracle: {1}: This creature can't be regenerated this turn. Only your opponents may activate this ability.
//! Set: LEG #6 — Legends | Scryfall ID: db1f578f-fa3b-4447-953b-1490852b6c80 | Oracle ID: 66566999-f70a-4f14-9bf0-23325295a977
// PARTIAL — both sentences are off the card: the destroy-replacement has no
// rule to carry it, and the {1} ability's activator restriction cannot be
// written.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CLERGY_OF_THE_HOLY_NIMBUS,
    oracle_id = "66566999-f70a-4f14-9bf0-23325295a977",
    scryfall_id = "db1f578f-fa3b-4447-953b-1490852b6c80",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Clergy of the Holy Nimbus",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "\"If this creature would be destroyed, regenerate it\" has no \
         ReplacementRule and no static Modifier for a regeneration shield, and \
         \"Only your opponents may activate this ability\" has no vocabulary — \
         no AbilityDef field or Activation setting names another player as the \
         one who may activate, so the whole {1} ability is off rather than \
         offered to the wrong player",
    ),
    // NOT SUPPORTED: "If this creature would be destroyed, regenerate it." —
    // `ReplacementRule` has no destroy-replacement variant, and a
    // regeneration shield is an `Effect` that only a resolving object can
    // create, never a standing static ability.
    // NOT SUPPORTED: "{1}: This creature can't be regenerated this turn. Only
    // your opponents may activate this ability." — the cost and
    // `Effect::CantBeRegeneratedThisTurn` are sayable, but the printed
    // activator is not: no field of `AbilityDef`/`ActivatedParts` names
    // anybody but the controller as the activator, and offering the ability
    // to this creature's controller would drop the restriction the card
    // prints.
);
