//! Secluded Starforge — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Tap X untapped artifacts you control: Target creature gets +X/+0 until end of turn. Activate only as a sorcery.
//! Oracle: {5}, {T}: Create a 2/2 colorless Robot artifact creature token.
//! Set: EOE #257 — Edge of Eternities | Scryfall ID: a997ff9f-045a-44a2-983d-f36414cef1ab | Oracle ID: 69f55a7c-6ddf-412e-b63b-b395731a1ff2
use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::SECLUDED_STARFORGE,
    oracle_id = "69f55a7c-6ddf-412e-b63b-b395731a1ff2",
    scryfall_id = "a997ff9f-045a-44a2-983d-f36414cef1ab",
    faces = &[face!(name = "Secluded Starforge", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf, TapX(&Filter::YOUR_ARTIFACT)),
            &[Effect::PumpTarget {
                keywords: KeywordSet::EMPTY,
                power: Amount::X,
                toughness: Amount::Fixed(0),
                duration: Duration::UntilEndOfTurn
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            timing = ActivationTiming::SorcerySpeed,
        ),
        activated!(
            cost!("{5}", TapSelf),
            &[Effect::CreateToken {
                token: &generated_tokens::ROBOT_ARTIFACT_2_2,
            }],
        ),
    ],
);
