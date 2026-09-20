//! Tectonic Edge — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: Destroy target nonbasic land. Activate only if an opponent controls four or more lands.
//! Set: C14 #313 — Commander 2014 | Scryfall ID: 94b9d7af-3e6f-4227-bb7d-a17d6f250535 | Oracle ID: 4927150d-7ff6-4232-b20e-d2ea245ac710
// PARTIAL — {T} for {C}, and the {1}, {T}, Sacrifice destruction of a
// nonbasic land. The activation restriction is not expressible, so that
// ability is ungated.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TECTONIC_EDGE,
    oracle_id = "4927150d-7ff6-4232-b20e-d2ea245ac710",
    scryfall_id = "94b9d7af-3e6f-4227-bb7d-a17d6f250535",
    faces = &[face!(name = "Tectonic Edge", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no Condition variant counts an opponent's permanents — ControlCount counts the ability's own controller's — so the destroy ability activates unconditionally"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "Activate only if an opponent controls four or more lands"
        activated!(
            cost!("{1}", TapSelf, SacrificeSelf),
            &[Effect::destroy(TargetSpec::Object(&Filter::NONBASIC_LAND))],
            target = Some(TargetSpec::Object(&Filter::NONBASIC_LAND)),
        ),
    ],
);
