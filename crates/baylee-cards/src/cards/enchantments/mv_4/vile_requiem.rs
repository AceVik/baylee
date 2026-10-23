//! Vile Requiem — {2}{B}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: {1}{B}, Sacrifice this enchantment: Destroy up to X target nonblack creatures, where X is the number of verse counters on this enchantment. They can't be regenerated.
//! Set: USG #167 — Urza's Saga | Scryfall ID: 2be44b37-2383-4bf4-ba30-d8d4e2cb8939 | Oracle ID: a72213e7-16f0-4fbe-91d3-750ca51a4336
// PARTIAL: targeting up to X nonblack creatures where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself to destroy up to one target nonblack creature without regeneration.

use baylee_cards_dsl::prelude::*;

static NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

card!(
    index = index::VILE_REQUIEM,
    oracle_id = "a72213e7-16f0-4fbe-91d3-750ca51a4336",
    scryfall_id = "2be44b37-2383-4bf4-ba30-d8d4e2cb8939",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "targeting up to X nonblack creatures where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself to destroy up to one target nonblack creature without regeneration",
    ),
    faces = &[face!(
        name = "Vile Requiem",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounterFilter {
                    filter: &Filter::This,
                    kind: counters::VERSE,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
        activated!(
            cost!("{1}{B}", SacrificeSelf),
            &[Effect::destroy_no_regen(TargetSpec::Object(
                &NONBLACK_CREATURE,
            ))],
            targets = Some(TargetReq::up_to_one(
                TargetSpec::Object(&NONBLACK_CREATURE,)
            )),
        ),
    ],
);
