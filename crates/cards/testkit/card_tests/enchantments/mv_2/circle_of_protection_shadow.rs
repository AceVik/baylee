//! `cards/enchantments/mv_2/circle_of_protection_shadow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn circle_of_protection_shadow() -> CardIndex {
    card_index("07c42f46-800e-4d2a-b844-c125ab93190d")
}

fn soltari_foot_soldier() -> CardIndex {
    card_index("6ccb3ac8-0509-431b-a196-d3cbf4fa80ae")
}

fn dauthi_mindripper() -> CardIndex {
    card_index("ecae5eba-8407-4c71-aeae-ab10fb1e5226")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// p0 has the Circle and two Plains; p1 has Soltari Foot Soldier (1/1
/// shadow), Dauthi Mindripper (2/1 shadow) and Grizzly Bears (2/2). p0's
/// first main phase.
fn a_circle_facing_two_shadows_and_a_bear() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[circle_of_protection_shadow(), plains(), plains()])
        .battlefield(
            1,
            &[soltari_foot_soldier(), dauthi_mindripper(), grizzly_bears()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Walks to p1's turn, attacks p0 with `attackers`, lets p0 block nothing, and
/// stops when p0 next holds priority (declare blockers, before damage).
#[track_caller]
fn p1_attacks_with(engine: &mut Engine<RegistryLookup>, attackers: &[CardIndex]) {
    let ids: Vec<ObjectId> = attackers
        .iter()
        .map(|&c| on_battlefield(engine, P1, c).unwrap())
        .collect();
    reach_their_main_phase(engine, P1);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: ids
                    .into_iter()
                    .map(|id| (id, Defender::Player(P0)))
                    .collect(),
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
}

/// Resolves the rest of combat while declining Mindripper's optional sacrifice.
/// The fixture must keep both shadow attackers for the damage comparison.
#[track_caller]
fn finish_combat_without_sacrificing(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..100 {
        if engine.state().turn.phase == Phase::SecondMain
            && matches!(engine.pending(), Pending::Priority { .. })
            && stack_is_empty(engine)
        {
            return;
        }
        if let Pending::YesNo { player, .. } = engine.pending().clone() {
            assert_eq!(player, P1, "Mindripper's controller chooses");
            engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
        } else {
            let (player, action) = answer_one(engine).expect("combat can advance");
            engine.apply(player, action).unwrap();
        }
    }
    panic!("combat did not finish: {:?}", engine.pending());
}

/// Pays {1}, lets the ability resolve to its question, and returns the
/// objects it offered.
#[track_caller]
fn activate_and_see_the_offer(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    tap_all_mana(engine, P0);
    activate(engine, P0, circle_of_protection_shadow(), 0);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0);
    options.iter().map(|source| source.object).collect()
}

/// Names `object` as the source, as it is offered right now.
#[track_caller]
fn choose_source(engine: &mut Engine<RegistryLookup>, object: ObjectId) {
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected the source question, got {:?}", engine.pending())
    };
    let source = *options
        .iter()
        .find(|source| source.object == object)
        .expect("the creature was on the offer");
    engine
        .apply(P0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
}

/// Circle of Protection: Shadow (07c42f46-800e-4d2a-b844-c125ab93190d) — {1}{W} Enchantment:
/// "{1}: The next time a creature of your choice with shadow would deal damage to you this turn, prevent that damage."
#[test]
fn circle_of_protection_shadow_prevents_the_chosen_shadow_creatures_damage_only() {
    let mut engine = a_circle_facing_two_shadows_and_a_bear();
    let soltari = on_battlefield(&engine, P1, soltari_foot_soldier()).unwrap();
    let mindripper = on_battlefield(&engine, P1, dauthi_mindripper()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    p1_attacks_with(
        &mut engine,
        &[soltari_foot_soldier(), dauthi_mindripper(), grizzly_bears()],
    );

    let offered = activate_and_see_the_offer(&mut engine);
    assert!(offered.contains(&soltari) && offered.contains(&mindripper));
    assert!(
        !offered.contains(&bears),
        "a creature without shadow cannot be chosen: {offered:?}"
    );
    choose_source(&mut engine, soltari);
    finish_combat_without_sacrificing(&mut engine);

    assert_eq!(
        engine.state().players[0].life,
        16,
        "Soltari's 1 is prevented; Mindripper's 2 and the Bears' 2 are not"
    );
    assert_eq!(engine.state().players[1].life, 20);
    assert!(on_battlefield(&engine, P1, dauthi_mindripper()).is_some());
}

#[test]
fn circle_of_protection_shadow_unused_lets_all_the_damage_through() {
    let mut engine = a_circle_facing_two_shadows_and_a_bear();
    p1_attacks_with(
        &mut engine,
        &[soltari_foot_soldier(), dauthi_mindripper(), grizzly_bears()],
    );

    finish_combat_without_sacrificing(&mut engine);

    assert_eq!(
        engine.state().players[0].life,
        15,
        "1 + 2 + 2 without the Circle"
    );
    assert!(on_battlefield(&engine, P1, dauthi_mindripper()).is_some());
}

/// "This turn": a shield made on p0's own turn is gone when p1 attacks.
#[test]
fn circle_of_protection_shadow_shield_does_not_outlast_the_turn() {
    let mut engine = a_circle_facing_two_shadows_and_a_bear();
    let soltari = on_battlefield(&engine, P1, soltari_foot_soldier()).unwrap();

    activate_and_see_the_offer(&mut engine);
    choose_source(&mut engine, soltari);
    pass_until(&mut engine, stack_is_empty);

    p1_attacks_with(&mut engine, &[soltari_foot_soldier()]);
    finish_combat_without_sacrificing(&mut engine);

    assert_eq!(
        engine.state().players[0].life,
        19,
        "the shield from the previous turn no longer prevents Soltari's damage"
    );
}
