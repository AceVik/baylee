//! The documented creature/player damage pair is one replacement event.

use crate::choice::{DamageEffectKind, Pending, PlayerAction};
use crate::engine::Engine;
use crate::engine::synthetic::{self, SyntheticLookup};
use baylee_cards_dsl::{AbilityDef, Amount, Effect, Filter, PlayerRel, TargetSpec};
use baylee_core::ids::{ObjectId, PlayerId};

const A: PlayerId = PlayerId::new(0);
const SOURCE: u32 = 98_095;
const BODY: u32 = 98_096;
const CREATURES: Effect = Effect::DealDamageEach {
    amount: Amount::Fixed(1),
    filter: &Filter::CREATURE,
};
const PLAYERS: Effect = Effect::DealDamage {
    amount: Amount::Fixed(1),
    target: TargetSpec::Player(PlayerRel::EachPlayer),
};
const AFTER: Effect = Effect::GainLife {
    amount: Amount::Fixed(7),
};

static ABILITIES: &[AbilityDef] = &[
    baylee_cards_dsl::activated!(baylee_cards_dsl::cost!(""), &[CREATURES, PLAYERS, AFTER]),
    baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!(""),
        &[CREATURES, Effect::TapSelf, PLAYERS, AFTER]
    ),
    baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!(""),
        &[Effect::RedirectNextFromChosenSource {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    ),
    baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!(""),
        &[Effect::PreventNextFromChosenSource {
            sources: &Filter::Any,
            combat_only: false,
            all_but: 0,
            gain_life: false,
        }]
    ),
    baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!(""),
        &[Effect::PreventNextDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    ),
];

type Bench = Engine<SyntheticLookup>;

fn walk_until(e: &mut Bench, ready: impl Fn(&Bench) -> bool) {
    for _ in 0..100 {
        if ready(e) {
            return;
        }
        let pending = e.pending().clone();
        assert!(synthetic::walk_past(e, &pending), "unexpected {pending:?}");
    }
    panic!("condition not reached: {:?}", e.pending());
}

fn settled(e: &mut Bench) {
    walk_until(e, |e| {
        e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == A)
    });
}

fn activate(e: &mut Bench, source: ObjectId, ability_index: u32) {
    e.apply(
        A,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .unwrap();
}

fn target_body(e: &mut Bench, body: ObjectId) {
    assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
    e.apply(
        A,
        PlayerAction::ChooseTargets {
            objects: vec![body],
            players: vec![],
        },
    )
    .unwrap();
}

fn choose_source(e: &mut Bench, source: ObjectId) {
    walk_until(e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = e.pending().clone()
    else {
        unreachable!();
    };
    let source = options.into_iter().find(|r| r.object == source).unwrap();
    e.apply(A, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
}

fn fixture() -> (Bench, ObjectId, ObjectId) {
    let cards = vec![
        synthetic::land(SOURCE, "Damage pair source", ABILITIES),
        synthetic::creature(BODY, "Redirected creature", 2, 2, &[]),
    ];
    let mut e = Engine::new(
        &synthetic::preset(98_095, &[SOURCE, BODY]),
        SyntheticLookup::new(cards),
    )
    .unwrap();
    while let Pending::Mulligan { player, .. } = e.pending().clone() {
        e.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    settled(&mut e);
    let source = synthetic::permanents(&e, SOURCE)[0];
    let body = synthetic::permanents(&e, BODY)[0];
    activate(&mut e, source, 2);
    target_body(&mut e, body);
    choose_source(&mut e, source);
    settled(&mut e);
    activate(&mut e, source, 3);
    choose_source(&mut e, source);
    settled(&mut e);
    // Competing prevention on the creature guarantees a real suspended
    // choice in both the paired and the separate-instruction scenarios.
    activate(&mut e, source, 4);
    target_body(&mut e, body);
    settled(&mut e);
    (e, source, body)
}

fn play_damage(separate: bool) {
    let (mut e, source, body) = fixture();
    let before = e.state().next_damage_batch;
    activate(&mut e, source, u32::from(separate));
    walk_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageEffect { .. })
    });
    assert_eq!(e.state().players[0].life, 20);
    assert_eq!(e.state().players[1].life, 20);
    let Pending::ChooseDamageEffect {
        player,
        choice,
        options,
        ..
    } = e.pending().clone()
    else {
        unreachable!();
    };
    let redirect = options
        .iter()
        .find(|option| matches!(option.kind, DamageEffectKind::Redirect { .. }))
        .unwrap();
    e.apply(
        player,
        PlayerAction::ChooseDamageEffect {
            choice,
            effect: redirect.id,
        },
    )
    .unwrap();
    settled(&mut e);
    assert_eq!(e.state().object(body).unwrap().damage, 0);
    assert_eq!(e.state().players[1].life, 19, "player damage runs once");
    assert_eq!(
        e.state().players[0].life,
        if separate { 26 } else { 27 },
        "one next-source shield covers both simultaneous parts, followed by exactly one gain of seven"
    );
    assert_eq!(
        e.state().next_damage_batch - before,
        if separate { 2 } else { 1 },
        "the continuation consumes exactly the intended damage operations"
    );
    assert_eq!(synthetic::tapped(&e, source), separate);
}

#[test]
fn redirected_creature_and_player_damage_share_one_next_source_shield() {
    play_damage(false);
}

#[test]
fn intervening_instruction_keeps_damage_events_separate_after_resuming() {
    play_damage(true);
}
