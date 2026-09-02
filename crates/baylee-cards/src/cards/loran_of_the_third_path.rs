//! Loran of the Third Path — {2}{W} — Legendary Creature — Human Artificer
//! Oracle: Vigilance
//! Oracle: When Loran enters, destroy up to one target artifact or enchantment.
//! Oracle: {T}: You and target opponent each draw a card.
//! Set: MKC #71 — Murders at Karlov Manor Commander | Scryfall ID: 9e83a0ef-4fea-45ba-86c0-130d6687f7fe | Oracle ID: b3d81980-76f2-44e2-b1c9-01e30c726312
// IMPLEMENTED — vigilance, ETB destroy, tap-draw for you and an opponent.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ARTIFACT_OR_ENCHANTMENT: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT]);

card! {
    index: 87,
    oracle_id: "b3d81980-76f2-44e2-b1c9-01e30c726312",
    scryfall_id: "9e83a0ef-4fea-45ba-86c0-130d6687f7fe",
    faces: &[face! {
        name: "Loran of the Third Path",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::HUMAN, creature::ARTIFICER],
        power: Some(2),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::VIGILANCE,
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::Destroy {
                target: TargetSpec::Object(&ARTIFACT_OR_ENCHANTMENT),
            }], targets: Some(TargetReq::up_to_one(TargetSpec::Object(
                &ARTIFACT_OR_ENCHANTMENT,
            )))),
        activated!(Cost::TAP, &[
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
                Effect::DrawCardsFor {
                    amount: Amount::Fixed(1),
                    who: PlayerRel::Opponent,
                },
            ]),
    ],
}
