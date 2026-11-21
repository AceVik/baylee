//! Teachings of the Kirin // Kirin-Touched Orochi, played through the Saga and both attack modes.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn teachings_of_the_kirin_transforms_and_its_attack_modes_reward_graveyard_exile() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let teachings = card_index("cb7edb5e-3a90-41c7-ae52-dc564086bf22");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[teachings, llanowar_elves()])
        .hand(1, &[llanowar_elves(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);
    let creature_card = hand_to_graveyard(&mut engine, them, llanowar_elves());
    let noncreature_card = hand_to_graveyard(&mut engine, them, plains());
    let opponent_elf = on_battlefield(&engine, them, llanowar_elves()).unwrap();

    let before_mill = library_size(&engine, me);
    cast_from_hand(&mut engine, me, teachings);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, me), before_mill - 3);
    let [spirit]: [ObjectId; 1] = tokens_of(&engine, me).try_into().expect("one Spirit");
    assert_spirit(&engine, spirit);

    reach_their_main_phase(&mut engine, them);
    assert_eq!(pass_until_targets(&mut engine, me), vec![spirit]);
    aim_at(&mut engine, me, spirit);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, spirit, CounterKind::P1P1), 1);

    reach_their_main_phase(&mut engine, them);
    reach_their_main_phase(&mut engine, me);
    pass_until(&mut engine, stack_is_empty);
    let orochi = on_battlefield(&engine, me, teachings).expect("the transformed Saga");
    assert_eq!(engine.state().object(orochi).unwrap().face_index, 1);

    reach_their_main_phase(&mut engine, them);
    reach_their_main_phase(&mut engine, me);
    attack_with_orochi(&mut engine, me, them, orochi, 0);
    assert_eq!(aim_at(&mut engine, me, creature_card), vec![creature_card]);
    pass_until(&mut engine, |engine| {
        engine.state().object(creature_card).unwrap().zone == Zone::Exile
    });
    assert_eq!(
        tokens_of(&engine, me),
        vec![spirit],
        "the token trigger uses the stack"
    );
    assert!(!stack_is_empty(&engine));
    pass_until(&mut engine, stack_is_empty);
    let [new_spirit]: [ObjectId; 1] = tokens_of(&engine, me)
        .into_iter()
        .filter(|&token| token != spirit)
        .collect::<Vec<_>>()
        .try_into()
        .expect("the creature card produced one more Spirit");
    assert_spirit(&engine, new_spirit);

    reach_their_main_phase(&mut engine, them);
    reach_their_main_phase(&mut engine, me);
    let other_creature_card = hand_to_graveyard(&mut engine, me, llanowar_elves());
    attack_with_orochi(&mut engine, me, them, orochi, 1);
    let options = aim_at(&mut engine, me, noncreature_card);
    assert!(options.contains(&noncreature_card));
    assert!(!options.contains(&other_creature_card));
    let counter_targets = pass_until_targets(&mut engine, me);
    assert_eq!(
        engine.state().object(noncreature_card).unwrap().zone,
        Zone::Exile
    );
    assert!(counter_targets.contains(&spirit));
    assert!(!counter_targets.contains(&opponent_elf));
    assert_eq!(counters_on(&engine, spirit, CounterKind::P1P1), 1);
    aim_at(&mut engine, me, spirit);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, spirit, CounterKind::P1P1), 2);
    assert_eq!(tokens_of(&engine, me).len(), 2);
}

fn attack_with_orochi(
    engine: &mut Engine<RegistryLookup>,
    me: PlayerId,
    them: PlayerId,
    orochi: ObjectId,
    mode: usize,
) {
    pass_until(
        engine,
        |engine| matches!(engine.pending(), Pending::ChooseAttackers { player, .. } if *player == me),
    );
    engine
        .apply(
            me,
            PlayerAction::DeclareAttackers {
                attackers: vec![(orochi, Defender::Player(them))],
            },
        )
        .unwrap();
    let Pending::ChooseCastMode { options, .. } = engine.pending() else {
        panic!(
            "the attack trigger should offer a mode, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(options.len(), 2);
    let slot = choose_cast_kind(engine, CastModeKind::Mode(mode));
    engine.apply(me, PlayerAction::ChooseMode(slot)).unwrap();
}

fn assert_spirit(engine: &Engine<RegistryLookup>, token: ObjectId) {
    let characteristics = engine.state().object(token).unwrap().characteristics();
    assert_eq!(pt(engine, token), (1, 1));
    assert!(characteristics.colors.is_empty());
    assert!(
        characteristics
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::SPIRIT)
    );
}
