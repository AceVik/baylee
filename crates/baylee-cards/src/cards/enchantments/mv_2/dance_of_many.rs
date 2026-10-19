//! Dance of Many — {U}{U} — Enchantment
//! Oracle: When this enchantment enters, create a token that's a copy of target nontoken creature.
//! Oracle: When this enchantment leaves the battlefield, exile the token.
//! Oracle: When the token leaves the battlefield, sacrifice this enchantment.
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {U}{U}.
//! Set: ME3 #34 — Masters Edition III | Scryfall ID: 54d5d755-403a-4e81-837e-f516eb17e819 | Oracle ID: 72e1a239-11a7-4dbe-b093-65d395e3ccc9

use baylee_cards_dsl::prelude::*;

static NONTOKEN_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::IsToken)]);

card!(
    index = index::DANCE_OF_MANY,
    oracle_id = "72e1a239-11a7-4dbe-b093-65d395e3ccc9",
    scryfall_id = "54d5d755-403a-4e81-837e-f516eb17e819",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dance of Many",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::CreateTokenCopyOfTargetLinkedBidirectionalLtb],
            targets = Some(TargetReq::one(TargetSpec::Object(&NONTOKEN_CREATURE)))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{U}{U}"),
                effect: &Effect::SacrificeSelf,
            }]
        ),
    ],
);
