//! Deepwood Elder — {G}{G} — Creature — Dryad Spellshaper
//! Oracle: {X}{G}{G}, {T}, Discard a card: X target lands become Forests until end of turn.
//! Set: MMQ #240 — Mercadian Masques | Scryfall ID: d2a1ed74-027e-4c8e-ac7e-e58c5fccff14 | Oracle ID: ffb62e4a-802d-4f1e-b92e-ec0919baafd5
// IMPLEMENTED — {X}{G}{G}, {T}, Discard a card turns X target lands into Forests until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEEPWOOD_ELDER,
    oracle_id = "ffb62e4a-802d-4f1e-b92e-ec0919baafd5",
    scryfall_id = "d2a1ed74-027e-4c8e-ac7e-e58c5fccff14",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Deepwood Elder",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRYAD, subtypes::creature::SPELLSHAPER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{X}{G}{G}", TapSelf, Discard(&Filter::Any)),
        &[Effect::ModifyTargets {
            modifier: Modifier::SetLandType(subtypes::land::FOREST),
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::LAND))),
    ),],
);
