//! Ignoble Hierarch — {G} — Creature — Goblin Shaman
//! Oracle: Exalted (Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.)
//! Oracle: {T}: Add {B}, {R}, or {G}.
//! Set: ECC #52 — Lorwyn Eclipsed Commander | Scryfall ID: e802cfe1-45e0-47cd-8745-363ccc0f2af8 | Oracle ID: c8de43a3-ebd3-4000-b343-a6ffed11d34d
// IMPLEMENTED — the three-colour mana ability; exalted is refused and noted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IGNOBLE_HIERARCH,
    oracle_id = "c8de43a3-ebd3-4000-b343-a6ffed11d34d",
    scryfall_id = "e802cfe1-45e0-47cd-8745-363ccc0f2af8",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    faces = &[face!(
        name = "Ignoble Hierarch",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::SHAMAN],
        power = Some(0),
        toughness = Some(1),
    ),],
    coverage =
        Coverage::Partial("exalted: no trigger or condition expresses a creature attacking alone"),
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ])]),
        // NOT SUPPORTED: Exalted — "Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn." `Trigger::Attacks` fires for every attacking creature and no `Condition` counts attackers, so "attacks alone" is a sentence the DSL cannot say.
    ],
);
