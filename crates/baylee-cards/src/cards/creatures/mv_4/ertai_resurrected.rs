//! Ertai Resurrected — {2}{U}{B} — Legendary Creature — Phyrexian Human Wizard
//! Oracle: Flash
//! Oracle: When Ertai Resurrected enters, choose up to one —
//! Oracle: • Counter target spell, activated ability, or triggered ability. Its controller draws a card.
//! Oracle: • Destroy another target creature or planeswalker. Its controller draws a card.
//! Set: DMU #199 — Dominaria United | Scryfall ID: 7f7e780e-fbc5-4dc0-b5c7-efcb8645c7c6 | Oracle ID: 3d038f7c-95fa-4b71-8f74-b9b4dd45cde0
// IMPLEMENTED — flash + modal ETB (counter / destroy / decline via the
// empty third mode for "up to one").

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static OTHER_CREATURE_OR_WALKER: Filter =
    Filter::And(&[Filter::Another, Filter::CREATURE_OR_PLANESWALKER]);
static COUNTER_EFFECTS: &[Effect] = &[
    Effect::CounterTargetSpellOrAbility,
    Effect::DrawCardsFor {
        amount: Amount::Fixed(1),
        who: PlayerRel::ControllerOfTarget,
    },
];
static DESTROY_EFFECTS: &[Effect] = &[
    Effect::destroy(TargetSpec::Object(&OTHER_CREATURE_OR_WALKER)),
    Effect::DrawCardsFor {
        amount: Amount::Fixed(1),
        who: PlayerRel::ControllerOfTarget,
    },
];

card!(
    index = index::ERTAI_RESURRECTED,
    oracle_id = "3d038f7c-95fa-4b71-8f74-b9b4dd45cde0",
    scryfall_id = "7f7e780e-fbc5-4dc0-b5c7-efcb8645c7c6",
    faces = &[face!(
        name = "Ertai Resurrected",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[creature::PHYREXIAN, creature::HUMAN, creature::WIZARD],
        power = Some(3),
        toughness = Some(2),
    )],
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::Black]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Implemented,
    abilities = &[modal_triggered!(
        Trigger::ETB,
        &[
            mode!(
                COUNTER_EFFECTS,
                targets = Some(TargetReq::one(TargetSpec::SpellOrAbility(&Filter::Any)))
            ),
            mode!(
                DESTROY_EFFECTS,
                targets = Some(TargetReq::one(TargetSpec::Object(
                    &OTHER_CREATURE_OR_WALKER
                )))
            ),
            // "Choose up to one" — declining is mode 2 (no effects).
            mode!(&[]),
        ]
    )],
);
