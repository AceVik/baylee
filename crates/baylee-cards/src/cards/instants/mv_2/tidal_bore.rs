//! Tidal Bore — {1}{U} — Instant
//! Oracle: You may return an Island you control to its owner's hand rather than pay this spell's mana cost.
//! Oracle: You may tap or untap target creature.
//! Set: MMQ #109 — Mercadian Masques | Scryfall ID: f68fd547-59fb-41e6-be55-1ec17fe2840b | Oracle ID: 513ad32a-2887-4825-adad-86a87cf1d2cb
// NOT SUPPORTED: Alternate cost of returning an Island you control to its owner's hand is not supported; spell effect works when cast for {1}{U}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIDAL_BORE,
    oracle_id = "513ad32a-2887-4825-adad-86a87cf1d2cb",
    scryfall_id = "f68fd547-59fb-41e6-be55-1ec17fe2840b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial("alternate return-an-Island cost not supported"),
    faces = &[face!(
        name = "Tidal Bore",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::MayDo {
            effects: &[Effect::ToggleTapTarget],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);
