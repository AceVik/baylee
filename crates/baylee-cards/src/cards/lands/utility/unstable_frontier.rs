//! Unstable Frontier — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Target land you control becomes the basic land type of your choice until end of turn.
//! Set: CON #145 — Conflux | Scryfall ID: d97e739f-8675-488e-be2b-4e455fbe390b | Oracle ID: 495214b5-2eab-4fe4-8879-a30a57a67163
// PARTIAL — {T}: Add {C} is built; the type-change ability is not expressible
// and is dropped (see the // NOT SUPPORTED line at the foot of the file).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNSTABLE_FRONTIER,
    oracle_id = "495214b5-2eab-4fe4-8879-a30a57a67163",
    scryfall_id = "d97e739f-8675-488e-be2b-4e455fbe390b",
    faces = &[face!(name = "Unstable Frontier", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no DSL variant sets a land's type to a basic land type chosen as the \
         ability resolves: a Modifier carries a fixed SubtypeId or \
         AllBasicLandTypes, and no Effect asks a player for a land type"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "{T}: Target land you control becomes the basic land type of
// your choice until end of turn." — the type change would be a
// `CreateContinuousEffect` on `Layer::Type` over `TargetSpec::Object(&YOUR_LAND)`,
// but `Modifier::AddSubtype` takes one printed subtype and
// `Modifier::AllBasicLandTypes` grants *every* basic land type at once; neither
// is "the basic land type of your choice", and nothing in `Effect` puts a
// subtype choice in front of the player on resolution (`EnterModifier::ChooseSubtype`
// only answers the question a permanent asks as it enters). The ability is
// dropped rather than approximated; Unstable Frontier keeps only its
// colourless mana ability.
