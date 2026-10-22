//! Seeds of Innocence — {1}{G}{G} — Sorcery
//! Oracle: Destroy all artifacts. They can't be regenerated. The controller of each of those artifacts gains life equal to its mana value.
//! Set: MIR #241 — Mirage | Scryfall ID: f9c868f5-0f90-4f7e-bafb-c45d2372fe06 | Oracle ID: f5ebad5c-9675-4a37-b76c-760416083939
// PARTIAL: Effect::DestroyAll does not track each destroyed artifact's mana value to give its controller that much life; destroys all artifacts with no regeneration.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEEDS_OF_INNOCENCE,
    oracle_id = "f5ebad5c-9675-4a37-b76c-760416083939",
    scryfall_id = "f9c868f5-0f90-4f7e-bafb-c45d2372fe06",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Effect::DestroyAll does not track each destroyed artifact's mana value to give its controller that much life; destroys all artifacts with no regeneration",
    ),
    faces = &[face!(
        name = "Seeds of Innocence",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::DestroyAll {
        filter: &Filter::ARTIFACT,
        no_regen: true,
    }])],
);
