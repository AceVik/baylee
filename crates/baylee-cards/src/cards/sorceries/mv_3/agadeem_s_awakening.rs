//! Agadeem's Awakening // Agadeem, the Undercrypt — {X}{B}{B}{B} — Sorcery // Land
//! Oracle: Return from your graveyard to the battlefield any number of target creature cards that each have a different mana value X or less.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: ZNR #90 — Zendikar Rising | Scryfall ID: 67f4c93b-080c-4196-b095-6a120a221988 | Oracle ID: 562d71b9-1646-474e-9293-55da6947a758
//! Face: Agadeem's Awakening — {X}{B}{B}{B} — Sorcery
//! Face: Agadeem, the Undercrypt —  — Land

use baylee_cards_dsl::prelude::*;

static TARGETS: TargetReq = TargetReq {
    distinct_mana_values: true,
    ..TargetReq::up_to(
        TargetSpec::CardInGraveyard(
            &Filter::And(&[Filter::CREATURE, Filter::CmcAtMostX]),
            PlayerRel::You,
        ),
        u8::MAX,
    )
};

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

card!(
    index = index::AGADEEM_S_AWAKENING,
    oracle_id = "562d71b9-1646-474e-9293-55da6947a758",
    scryfall_id = "67f4c93b-080c-4196-b095-6a120a221988",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::GraveyardToBattlefield {
            target: TARGETS.spec,
            owner_control: false,
            counters: None,
        }],
        targets = Some(TARGETS),
    )],
    faces = &[
        face!(
            name = "Agadeem's Awakening",
            mana_cost = mana!("{X}{B}{B}{B}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Agadeem, the Undercrypt",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
            abilities = BACK_MANA,
        ),
    ],
);
