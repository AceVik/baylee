//! Growing Rites of Itlimoc // Itlimoc, Cradle of the Sun — {2}{G} — Legendary Enchantment // Legendary Land
//! Oracle: When Growing Rites of Itlimoc enters, look at the top four cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on the bottom of your library in any order.
//! Oracle: At the beginning of your end step, if you control four or more creatures, transform Growing Rites of Itlimoc.
//! Oracle: (Transforms from Growing Rites of Itlimoc.)
//! Oracle: {T}: Add {G}.
//! Oracle: {T}: Add {G} for each creature you control.
//! Set: LCI #188 — The Lost Caverns of Ixalan | Scryfall ID: 004524bf-b249-4dac-9c10-44d57143feb9 | Oracle ID: ea9c459a-6047-43aa-968f-a582be4000e8
//! Face: Growing Rites of Itlimoc — {2}{G} — Legendary Enchantment
//! Face: Itlimoc, Cradle of the Sun —  — Legendary Land
// IMPLEMENTED — end-step transform at four or more creatures, and the back
// face's {G} and {G}-per-creature mana abilities; the enters trigger is
// dropped, see the NOT SUPPORTED note below.

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
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "the enters trigger: Effect::LookAtTopPick carries a count and a pick and \
         neither a filter nor a \"you may\", so \"reveal a creature card from \
         among them\" cannot be said — it would put any of the four into hand",
    ),
    abilities = &[
        // NOT SUPPORTED: "When Growing Rites of Itlimoc enters, look at the top
        // four cards of your library. You may reveal a creature card from among
        // them and put it into your hand. Put the rest on the bottom of your
        // library in any order." — the ability is off the card, because the only
        // look-at-top effect in the DSL takes no filter (it would take any of the
        // four) and states no optional pick.
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            &[Effect::ExileSelfReturnAsFace { face: 1 }],
            condition = Some(Condition::ControlCount(&Filter::CREATURE, 4)),
        ),
    ],
);
