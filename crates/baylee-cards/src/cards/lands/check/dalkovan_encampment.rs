//! Dalkovan Encampment — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Swamp or a Mountain.
//! Oracle: {T}: Add {W}.
//! Oracle: {2}{W}, {T}: Whenever you attack this turn, create two 1/1 red Warrior creature tokens that are tapped and attacking. Sacrifice them at the beginning of the next end step.
//! Set: TDM #253 — Tarkir: Dragonstorm | Scryfall ID: 98ad5f0c-8775-4e89-8e92-84a6ade93e35 | Oracle ID: 33a90122-7280-4481-9b97-5879194cae40
// PARTIAL — the enters-tapped-unless clause (EnterModifier::TappedUnless) and
// {T}: Add {W} are built; the {2}{W} activation is not expressible (see the
// NOT SUPPORTED line beside the abilities).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SWAMP_OR_MOUNTAIN: Filter = Filter::And(&[
    Filter::Or(&[
        Filter::HasSubtype(land::SWAMP),
        Filter::HasSubtype(land::MOUNTAIN),
    ]),
    Filter::ControlledByYou,
]);

card!(
    index = index::DALKOVAN_ENCAMPMENT,
    oracle_id = "33a90122-7280-4481-9b97-5879194cae40",
    scryfall_id = "98ad5f0c-8775-4e89-8e92-84a6ade93e35",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Dalkovan Encampment",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&SWAMP_OR_MOUNTAIN)],
    )],
    coverage = Coverage::Partial(
        "the {2}{W}, {T} activation: a delayed \"whenever you attack this turn\" \
         trigger that creates two 1/1 red Warrior tokens tapped and attacking and \
         sacrifices them at the beginning of the next end step",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        // NOT SUPPORTED: "{2}{W}, {T}: Whenever you attack this turn, create two
        // 1/1 red Warrior creature tokens that are tapped and attacking.
        // Sacrifice them at the beginning of the next end step." — no Effect
        // registers a delayed trigger on attacking this turn (only
        // DelayedManaAtNextFirstMain and PayCostOrLoseLater carry anything past
        // the resolution), CreateToken/CreateTokenN have no "tapped and
        // attacking" flags, and nothing schedules a sacrifice at the beginning
        // of the next end step. The ledger also holds no Warrior token, and a
        // card file may not define one (crate::tokens owns every id).
    ],
);
