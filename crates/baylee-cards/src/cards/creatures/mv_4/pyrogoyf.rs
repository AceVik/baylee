//! Pyrogoyf — {3}{R} — Creature — Lhurgoyf
//! Oracle: Pyrogoyf's power is equal to the number of card types among cards in all graveyards and its toughness is equal to that number plus 1.
//! Oracle: Whenever this creature or another Lhurgoyf creature you control enters, that creature deals damage equal to its power to any target.
//! Set: AA2 #12 — Arena Anthology 2 | Scryfall ID: 7802a658-1d55-4cb7-baa9-eb53e534f723 | Oracle ID: 7fd7457a-388d-4cca-a7cf-86b4ea922037
// IMPLEMENTED — a characteristic-defining P/T over the card types in all
// graveyards (CR 604.3, layer 7a), and a Lhurgoyf entering under its
// controller's control deals damage equal to its own power to any target.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PYROGOYF,
    oracle_id = "7fd7457a-388d-4cca-a7cf-86b4ea922037",
    scryfall_id = "7802a658-1d55-4cb7-baa9-eb53e534f723",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Pyrogoyf",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LHURGOYF],
        power = Some(0),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CharacteristicPT {
                count: PtCount::CardTypesInAllGraveyards,
                toughness_plus: 1,
            }
        ),
        // "this creature or another Lhurgoyf creature you control": Pyrogoyf
        // is itself a Lhurgoyf, and `This` keeps it in the sentence if an
        // effect ever takes the subtype away. The damage comes from the
        // creature that entered, not from Pyrogoyf.
        triggered!(
            Trigger::EntersBattlefield(&Filter::Or(&[
                Filter::This,
                Filter::And(&[
                    Filter::YOUR_CREATURE,
                    Filter::HasSubtype(subtypes::creature::LHURGOYF),
                ]),
            ])),
            &[Effect::EventObjectDealsDamageEqualToPower {
                target: TargetSpec::AnyTarget,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
    ],
);
