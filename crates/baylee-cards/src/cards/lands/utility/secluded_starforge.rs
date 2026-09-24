//! Secluded Starforge — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Tap X untapped artifacts you control: Target creature gets +X/+0 until end of turn. Activate only as a sorcery.
//! Oracle: {5}, {T}: Create a 2/2 colorless Robot artifact creature token.
//! Set: EOE #257 — Edge of Eternities | Scryfall ID: a997ff9f-045a-44a2-983d-f36414cef1ab | Oracle ID: 69f55a7c-6ddf-412e-b63b-b395731a1ff2
// PARTIAL — {T}: Add {C} and the Robot are built; the pump comes off the
// card, with a `// NOT SUPPORTED:` saying what the vocabulary cannot say.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::SECLUDED_STARFORGE,
    oracle_id = "69f55a7c-6ddf-412e-b63b-b395731a1ff2",
    scryfall_id = "a997ff9f-045a-44a2-983d-f36414cef1ab",
    faces = &[face!(name = "Secluded Starforge", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the \"Tap X untapped artifacts you control\" cost has no spelling: \
         CostPart::TapOther names one permanent and carries no count",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{2}, {T}, Tap X untapped artifacts you control:
        // Target creature gets +X/+0 until end of turn. Activate only as a
        // sorcery." — the cost cannot be said. CostPart::TapOther takes a
        // filter and names exactly one permanent; no cost part carries a
        // count, so writing it would ship "tap one artifact" beside a pump
        // reading an X that no activation ever announced.
        activated!(
            cost!("{5}", TapSelf),
            &[Effect::CreateToken {
                token: &generated_tokens::ROBOT_ARTIFACT_2_2,
            }],
        ),
    ],
);
