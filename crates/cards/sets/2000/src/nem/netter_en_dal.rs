//! Netter en-Dal — {W} — Creature — Human Spellshaper
//! Oracle: {W}, {T}, Discard a card: Target creature can't attack this turn.
//! Set: NEM #13 — Nemesis | Scryfall ID: 2190649d-f898-4693-8ccf-80e6709d8496 | Oracle ID: 8cd65dcd-e36d-47b1-b011-961c79004cb0
// IMPLEMENTED — activated ability ({W}, {T}, Discard a card: Target creature can't attack this turn).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NETTER_EN_DAL,
    oracle_id = "8cd65dcd-e36d-47b1-b011-961c79004cb0",
    scryfall_id = "2190649d-f898-4693-8ccf-80e6709d8496",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Netter en-Dal",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SPELLSHAPER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{W}", TapSelf, Discard(&Filter::Any)),
        &[Effect::ModifyTargets {
            modifier: Modifier::CantAttack,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    ),],
);
