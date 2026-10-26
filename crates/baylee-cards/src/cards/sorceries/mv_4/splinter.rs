//! Splinter — {2}{G}{G} — Sorcery
//! Oracle: Exile target artifact. Search its controller's graveyard, hand, and library for all cards with the same name as that artifact and exile them. Then that player shuffles.
//! Set: BOK #146 — Betrayers of Kamigawa | Scryfall ID: 5bcc210b-e37e-463c-8fd4-83e5113429a9 | Oracle ID: 90f01e9f-68f1-4e41-97dc-8de990a91504
// NOT SUPPORTED: searching the artifact's controller's graveyard, hand, and library for all cards with the same name and exiling them is not in Effect.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPLINTER,
    oracle_id = "90f01e9f-68f1-4e41-97dc-8de990a91504",
    scryfall_id = "5bcc210b-e37e-463c-8fd4-83e5113429a9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "exiles target artifact; searching its controller's graveyard, hand, and library for all cards with the same name and exiling them is not in Effect"
    ),
    faces = &[face!(
        name = "Splinter",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::exile(TargetSpec::Object(&Filter::ARTIFACT))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
    )],
);
