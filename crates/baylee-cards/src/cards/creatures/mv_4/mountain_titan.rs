//! Mountain Titan — {2}{B}{R} — Creature — Giant
//! Oracle: {1}{R}{R}: Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature.
//! Set: ICE #299 — Ice Age | Scryfall ID: bcc1d589-02a2-4896-a283-9d0385534667 | Oracle ID: 160c8e94-ccf9-4058-b7fd-04529456f0fa
// IMPLEMENTED — {1}{R}{R} activated ability granting a triggered ability until end of turn that puts a +1/+1 counter on Mountain Titan whenever you cast a black spell.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_BLACK_SPELL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

card!(
    index = index::MOUNTAIN_TITAN,
    oracle_id = "160c8e94-ccf9-4058-b7fd-04529456f0fa",
    scryfall_id = "bcc1d589-02a2-4896-a283-9d0385534667",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mountain Titan",
        mana_cost = mana!("{2}{B}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{1}{R}{R}"),
        &[Effect::continuous(
            &Filter::This,
            Modifier::GrantTriggered {
                trigger: Trigger::SpellCast(&YOUR_BLACK_SPELL),
                effects: &[Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                }],
                target: None,
            },
            Duration::UntilEndOfTurn,
        )],
    )],
);
