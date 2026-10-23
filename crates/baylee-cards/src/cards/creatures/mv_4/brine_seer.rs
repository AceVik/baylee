//! Brine Seer — {3}{U} — Creature — Human Wizard
//! Oracle: {2}{U}, {T}: Reveal any number of blue cards in your hand. Counter target spell unless its controller pays {1} for each card revealed this way.
//! Set: UDS #28 — Urza's Destiny | Scryfall ID: 2f6e5575-b004-417f-9366-6ba7840a79e7 | Oracle ID: 64c88c94-3a9d-4fd4-b58c-4d2167c2b8d5
// PARTIAL: revealing any number of blue cards in your hand as part of resolution is not in Effect; activates for {2}{U}, {T} to counter target spell unless its controller pays {1} for each blue card in your hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLUE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Blue]));

card!(
    index = index::BRINE_SEER,
    oracle_id = "64c88c94-3a9d-4fd4-b58c-4d2167c2b8d5",
    scryfall_id = "2f6e5575-b004-417f-9366-6ba7840a79e7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "revealing any number of blue cards in your hand as part of resolution is not in Effect; activates for {2}{U}, {T} to counter target spell unless its controller pays {1} for each blue card in your hand",
    ),
    faces = &[face!(
        name = "Brine Seer",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{2}{U}", TapSelf),
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::CountOf {
                filter: &BLUE_CARD,
                zone: ZoneSel::HandYou,
            },
            effect: &Effect::CounterTargetSpell,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )],
);
