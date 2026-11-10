//! Martyr's Cause — {2}{W} — Enchantment
//! Oracle: Sacrifice a creature: The next time a source of your choice would deal damage to any target this turn, prevent that damage.
//! Set: ULG #13 — Urza's Legacy | Scryfall ID: 4c1f026b-8c7f-4051-9922-5684a6b2c06b | Oracle ID: 72d1c789-e79f-4f15-80d0-981d4120b085
// PARTIAL — only the controller can be targeted; prevention for other recipients is unsupported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MARTYR_S_CAUSE,
    oracle_id = "72d1c789-e79f-4f15-80d0-981d4120b085",
    scryfall_id = "4c1f026b-8c7f-4051-9922-5684a6b2c06b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Sacrifice a creature: target yourself to prevent the next damage a chosen source would deal to you this turn; prevention for other recipients is unsupported.",
    ),
    faces = &[face!(
        name = "Martyr's Cause",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: Prevention for target creatures and other players is unavailable; only targeting yourself is supported.
    abilities = &[activated!(
        cost!(Sacrifice(&Filter::YOUR_CREATURE)),
        &[Effect::PreventNextFromChosenSource {
            sources: &Filter::Any,
            combat_only: false,
            all_but: 0,
            gain_life: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Player(PlayerRel::You))),
    )],
);
