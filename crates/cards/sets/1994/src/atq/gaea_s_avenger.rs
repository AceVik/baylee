//! Gaea's Avenger — {1}{G}{G} — Creature — Treefolk
//! Oracle: Gaea's Avenger's power and toughness are each equal to 1 plus the number of artifacts your opponents control.
//! Set: ME4 #155 — Masters Edition IV | Scryfall ID: 5df83b52-02df-4dfb-87b5-e2eab3d6c65b | Oracle ID: a4b60080-3f80-4aeb-8bd9-7953e77aaf75

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GAEA_S_AVENGER,
    oracle_id = "a4b60080-3f80-4aeb-8bd9-7953e77aaf75",
    scryfall_id = "5df83b52-02df-4dfb-87b5-e2eab3d6c65b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gaea's Avenger",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPT {
            count: PtCount::Plus {
                base: &PtCount::OnBattlefield(&Filter::And(&[
                    Filter::ARTIFACT,
                    Filter::ControlledByOpponent,
                ])),
                offset: 1
            },
            toughness_plus: 0,
        }
    )],
);
