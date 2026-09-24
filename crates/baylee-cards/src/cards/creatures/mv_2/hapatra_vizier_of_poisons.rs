//! Hapatra, Vizier of Poisons — {B}{G} — Legendary Creature — Human Cleric
//! Oracle: Whenever Hapatra deals combat damage to a player, you may put a -1/-1 counter on target creature.
//! Oracle: Whenever you put one or more -1/-1 counters on a creature, create a 1/1 green Snake creature token with deathtouch.
//! Set: ECC #123 — Lorwyn Eclipsed Commander | Scryfall ID: d19e17fc-ae3e-4533-ab94-70d5f2e1b8cd | Oracle ID: a8ddea1c-8d80-49c1-a5b4-630d5e51d66e
// PARTIAL — the combat-damage trigger puts a -1/-1 counter on up to one
// target creature ("you may" is the target count); the second trigger has
// no DSL variant.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HAPATRA_VIZIER_OF_POISONS,
    oracle_id = "a8ddea1c-8d80-49c1-a5b4-630d5e51d66e",
    scryfall_id = "d19e17fc-ae3e-4533-ab94-70d5f2e1b8cd",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Hapatra, Vizier of Poisons",
        mana_cost = mana!("{B}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial(
        "no Trigger fires on -1/-1 counters being put on a creature, and the counter event does not record who put them"
    ),
    abilities = &[triggered!(
        Trigger::DealsCombatDamageToPlayer(&Filter::This),
        &[Effect::AddCounter {
            kind: CounterKind::M1M1,
            amount: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);

// NOT SUPPORTED: "Whenever you put one or more -1/-1 counters on a creature,
// create a 1/1 green Snake creature token with deathtouch." — no `Trigger`
// fires on counters being placed on a permanent (`TriggerEventKind` has only
// `EntersBattlefield`/`Any`, and replaces trigger *rules* rather than
// counters), and `GameEvent::CounterChanged` does not record who put the
// counters, which "whenever you put" needs. The Snake token exists.
