//! Growing Rites of Itlimoc // Itlimoc, Cradle of the Sun — {2}{G} — Legendary Enchantment // Legendary Land
//! Oracle: When Growing Rites of Itlimoc enters, look at the top four cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on the bottom of your library in any order.
//! Oracle: At the beginning of your end step, if you control four or more creatures, transform Growing Rites of Itlimoc.
//! Oracle: (Transforms from Growing Rites of Itlimoc.)
//! Oracle: {T}: Add {G}.
//! Oracle: {T}: Add {G} for each creature you control.
//! Set: LCI #188 — The Lost Caverns of Ixalan | Scryfall ID: 004524bf-b249-4dac-9c10-44d57143feb9 | Oracle ID: ea9c459a-6047-43aa-968f-a582be4000e8
//! Face: Growing Rites of Itlimoc — {2}{G} — Legendary Enchantment
//! Face: Itlimoc, Cradle of the Sun —  — Legendary Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
    mana_ability!(&[Effect::mana_dynamic(
        ManaColor::Green,
        Amount::CountOf {
            filter: &Filter::YOUR_CREATURE,
            zone: ZoneSel::Battlefield,
        },
    )]),
];

card!(
    index = index::GROWING_RITES_OF_ITLIMOC,
    oracle_id = "ea9c459a-6047-43aa-968f-a582be4000e8",
    scryfall_id = "004524bf-b249-4dac-9c10-44d57143feb9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Growing Rites of Itlimoc",
            mana_cost = mana!("{2}{G}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Itlimoc, Cradle of the Sun",
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::LookAtTopFind {
                found: Find::HAND,
                count: 4,
                filter: &Filter::CREATURE,
                random: false
            }]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            &[Effect::TransformSource],
            condition = Some(Condition::ControlCount(&Filter::CREATURE, 4)),
        ),
    ],
);
