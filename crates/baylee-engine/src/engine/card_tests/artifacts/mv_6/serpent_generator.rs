//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as kit;
use baylee_core::generated::index;

#[test]
fn serpent_generator_poison_triggers_on_any_positive_player_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::SERPENT_GENERATOR])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, p0, index::SERPENT_GENERATOR, 0);
    pass_until(&mut e, stack_is_empty);
    let snake = crate::engine::card_tests::token_helpers::named(&e, "Snake")[0];
    assert_eq!(pt(&e, snake), (1, 1));
    assert!(
        e.state()
            .object(snake)
            .unwrap()
            .characteristics()
            .colors
            .is_colorless()
    );
    for (recipient, amount, expected) in [(p1, 0, 0), (p1, 3, 1), (p0, 1, 1)] {
        crate::engine::card_tests::history_helpers::damage_player(
            &mut e, snake, recipient, amount, false,
        );
        e.apply(p0, PlayerAction::PassPriority).unwrap();
        pass_until(&mut e, |e| {
            stack_is_empty(e) && matches!(e.pending(),Pending::Priority{player,..}if *player==p0)
        });
        assert_eq!(e.state().players[recipient.get() as usize].poison, expected);
    }
}

/// Damage from `source` to the creature `recipient`, through the same door
/// [`history_helpers::damage_player`] uses for a player.
fn damage_creature(e: &mut Engine<RegistryLookup>, source: ObjectId, recipient: ObjectId, n: u32) {
    let state = e.dev_state_mut(PlayerId::new(0)).unwrap();
    let source_version = Some(state.object(source).unwrap().version);
    let mut work = crate::damage::DamageWork::new(
        state,
        vec![crate::damage::Assignment {
            source,
            source_version,
            recipient: crate::event::DamageTarget::Object(recipient),
            amount: n,
            is_combat: false,
        }],
    );
    assert!(work.advance(state).is_none());
}

/// Lets a damage event's triggers go on the stack and resolve.
fn settle_damage(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    e.apply(seat, PlayerAction::PassPriority).unwrap();
    pass_until(e, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
    });
}

/// "Create a 1/1 colorless Snake artifact creature token": the token is an
/// artifact AND a creature, and "deals damage to a player" is not "deals
/// damage": a Snake hurting a creature gives nobody a poison counter.
#[test]
fn serpent_generator_snake_is_an_artifact_creature_and_hurting_a_creature_poisons_nobody() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = kit::game(
        SEED,
        &[index::SERPENT_GENERATOR],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    kit::float(&mut e, p0, &[(ManaColor::Colorless, 4)]);
    activate(&mut e, p0, index::SERPENT_GENERATOR, 0);
    pass_until(&mut e, stack_is_empty);
    let snake = crate::engine::card_tests::token_helpers::named(&e, "Snake")[0];
    let types = e.state().object(snake).unwrap().characteristics().types;
    assert!(
        types.contains(TypeSet::ARTIFACT) && types.contains(TypeSet::CREATURE),
        "\"Snake artifact creature token\" is both an artifact and a creature: {types:?}"
    );

    let bears = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    damage_creature(&mut e, snake, bears, 1);
    settle_damage(&mut e, p0);
    assert_eq!(
        e.state().players[0].poison + e.state().players[1].poison,
        0,
        "the poison clause names damage dealt to a player; a creature is not one"
    );
    assert_eq!(
        kit::damage_on(&e, bears),
        1,
        "the damage to the creature really was dealt"
    );
}

/// "Whenever THIS creature deals damage to a player": another creature
/// hurting a player is not the Snake.
#[test]
fn serpent_generator_snake_poisons_only_for_its_own_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = kit::game(
        SEED,
        &[index::SERPENT_GENERATOR, index::GRIZZLY_BEARS],
        &[],
        &[],
        &[],
    );
    kit::float(&mut e, p0, &[(ManaColor::Colorless, 4)]);
    activate(&mut e, p0, index::SERPENT_GENERATOR, 0);
    pass_until(&mut e, stack_is_empty);
    let snake = crate::engine::card_tests::token_helpers::named(&e, "Snake")[0];
    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();

    crate::engine::card_tests::history_helpers::damage_player(&mut e, bears, p1, 2, false);
    settle_damage(&mut e, p0);
    assert_eq!(
        e.state().players[1].poison,
        0,
        "a Bear that is not the Snake carries no poison"
    );
    assert_eq!(kit::life(&e, p1), 18, "the Bear's damage was dealt");

    crate::engine::card_tests::history_helpers::damage_player(&mut e, snake, p1, 2, false);
    settle_damage(&mut e, p0);
    assert_eq!(
        e.state().players[1].poison,
        1,
        "the Snake's own damage to a player does give one counter"
    );
}

/// "{4}, {T}": the whole of {4} is spent, and the Generator itself taps, so
/// a second Snake needs the Generator untapped as well as four more mana.
#[test]
fn serpent_generator_costs_four_mana_and_taps_itself() {
    let p0 = PlayerId::new(0);
    let mut e = kit::game(SEED, &[index::SERPENT_GENERATOR], &[], &[], &[]);
    let generator = kit::obj(&e, p0, index::SERPENT_GENERATOR);
    kit::float(&mut e, p0, &[(ManaColor::Colorless, 3)]);
    assert!(
        !kit::offered(&e, index::SERPENT_GENERATOR, p0, 0),
        "{{4}}: three mana is one short"
    );
    kit::float(&mut e, p0, &[(ManaColor::Colorless, 1)]);
    assert!(kit::offered(&e, index::SERPENT_GENERATOR, p0, 0));
    activate(&mut e, p0, index::SERPENT_GENERATOR, 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(kit::pool_total(&e, p0), 0, "all of {{4}} was paid");
    assert!(kit::tapped(&e, generator), "{{T}} is part of the cost");
    assert_eq!(
        crate::engine::card_tests::token_helpers::named(&e, "Snake").len(),
        1
    );

    kit::float(&mut e, p0, &[(ManaColor::Colorless, 4)]);
    assert!(
        !kit::offered(&e, index::SERPENT_GENERATOR, p0, 0),
        "a tapped Generator cannot pay {{T}} again, however much mana floats"
    );
    e.dev_state_mut(p0).unwrap().set_tapped(generator, false);
    e.refresh_offer();
    assert!(
        kit::offered(&e, index::SERPENT_GENERATOR, p0, 0),
        "untapped, with four mana, it is offered again"
    );
}
