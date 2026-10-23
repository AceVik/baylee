//! Discordant Dirge — {3}{B}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: {B}, Sacrifice this enchantment: Look at target opponent's hand and choose up to X cards from it, where X is the number of verse counters on this enchantment. That player discards those cards.
//! Set: USG #131 — Urza's Saga | Scryfall ID: d48e753c-ba11-4aa3-9a73-584f8a7538f5 | Oracle ID: edd4741b-3ee6-421a-8bac-d60736755356
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISCORDANT_DIRGE,
    oracle_id = "edd4741b-3ee6-421a-8bac-d60736755356",
    scryfall_id = "d48e753c-ba11-4aa3-9a73-584f8a7538f5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "choosing up to X cards from an opponent's hand where X is the verse-counter count is not in Effect; implements the upkeep optional verse counter and a single-card revealed-hand discard on {B}, Sacrifice",
    ),
    faces = &[face!(
        name = "Discordant Dirge",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounter {
                    kind: counters::VERSE,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
        activated!(
            cost!("{B}", SacrificeSelf),
            &[Effect::RevealHandDiscard {
                filter: &Filter::Any,
            }],
            target = Some(TargetSpec::AnyOpponent),
        ),
    ],
);
