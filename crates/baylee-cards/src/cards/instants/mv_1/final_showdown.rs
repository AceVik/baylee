//! Final Showdown — {W} — Instant
//! Oracle: Spree (Choose one or more additional costs.)
//! Oracle: + {1} — All creatures lose all abilities until end of turn.
//! Oracle: + {1} — Choose a creature you control. It gains indestructible until end of turn.
//! Oracle: + {3}{W}{W} — Destroy all creatures.
//! Set: OTJ #11 — Outlaws of Thunder Junction | Scryfall ID: 358968f9-45bd-4022-b6bc-f1f7e0adf0e7 | Oracle ID: 7e7ec3d6-a84f-4cc3-93f4-4d181d41e126

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FINAL_SHOWDOWN,
    oracle_id = "7e7ec3d6-a84f-4cc3-93f4-4d181d41e126",
    scryfall_id = "358968f9-45bd-4022-b6bc-f1f7e0adf0e7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Final Showdown",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    // Spree (CR 702.172a): one or more modes, each mode's cost added to
    // {W}. In printed order (CR 608.2c): the abilities go first, so the
    // indestructible granted after them stays, and the creature it was
    // granted to survives the destruction that follows.
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE_OR_MORE,
        modes: &[
            mode!(
                &[Effect::continuous(
                    &Filter::CREATURE,
                    Modifier::LoseAllAbilities,
                    Duration::UntilEndOfTurn
                )],
                additional_cost = Some(mana!("{1}"))
            ),
            mode!(
                &[Effect::ChooseYoursThen {
                    filter: &Filter::YOUR_CREATURE,
                    then: &[Effect::continuous(
                        &Filter::This,
                        Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE),
                        Duration::UntilEndOfTurn
                    )],
                }],
                additional_cost = Some(mana!("{1}"))
            ),
            mode!(
                &[Effect::destroy_all(&Filter::CREATURE)],
                additional_cost = Some(mana!("{3}{W}{W}"))
            ),
        ],
    }],
);
