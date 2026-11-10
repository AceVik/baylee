//! `cards/creatures/mv_2/argothian_pixies.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "bbf183bc-d502-4432-8202-f29f60c08396"
fn argothian_pixies() -> CardIndex {
    card_index("bbf183bc-d502-4432-8202-f29f60c08396")
}

/// Damage marked on `id` right now (cleanup wipes it).
fn marked(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine.state().object(id).expect("on the table").damage
}

/// A 2/1 for {1}{G}.
#[test]
fn argothian_pixies_is_the_two_one_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[argothian_pixies()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pixies = on_battlefield(&engine, p0, argothian_pixies()).expect("seated");
    assert_eq!(pt(&engine, pixies), (2, 1));
}

/// "This creature can't be blocked by artifact creatures." Two artifact
/// creatures (Su-Chi, Steel Wall) and a plain Grizzly Bears face the Pixies:
/// only the Bears are offered as blockers.
#[test]
fn argothian_pixies_cant_be_blocked_by_artifact_creatures_but_by_others() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[argothian_pixies()])
        .battlefield(1, &[su_chi(), steel_wall(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pixies = on_battlefield(&engine, p0, argothian_pixies()).expect("seated");
    let su_chi_id = on_battlefield(&engine, p1, su_chi()).expect("seated");
    let wall = on_battlefield(&engine, p1, steel_wall()).expect("seated");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");

    let offered = attack_and_collect_blocks(&mut engine, pixies, p1);
    let can_block = |id: ObjectId| {
        offered
            .iter()
            .any(|b| b.blocker == id && b.attackers.contains(&pixies))
    };
    assert!(can_block(bears), "a nonartifact creature may block it");
    assert!(!can_block(su_chi_id), "an artifact creature may not");
    assert!(
        !can_block(wall),
        "an artifact creature may not (Steel Wall)"
    );
}

/// The opponent attacks with `attacker`; the Pixies block it. Returns the
/// engine after combat damage, within the turn.
fn pixies_block(attacker_card: CardIndex) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[argothian_pixies()])
        .battlefield(1, &[attacker_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pixies = on_battlefield(&engine, p0, argothian_pixies()).expect("seated");
    let attacker = on_battlefield(&engine, p1, attacker_card).expect("seated");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(pixies, attacker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    (engine, pixies, attacker)
}

/// "Prevent all damage that would be dealt to this creature by artifact
/// creatures": Su-Chi's 4 never lands on the 2/1, which deals its 2 back.
#[test]
fn argothian_pixies_takes_no_combat_damage_from_an_artifact_creature() {
    let (engine, pixies, su_chi_id) = pixies_block(su_chi());
    assert_eq!(marked(&engine, pixies), 0);
    assert!(
        on_battlefield(&engine, PlayerId::new(0), argothian_pixies()).is_some(),
        "the Pixies survive"
    );
    assert_eq!(marked(&engine, su_chi_id), 2);
}

/// Control: a nonartifact creature's combat damage kills the Pixies.
#[test]
fn argothian_pixies_dies_to_a_nonartifact_creature() {
    let (engine, _pixies, _bears) = pixies_block(grizzly_bears());
    assert!(
        in_graveyard(&engine, PlayerId::new(0), argothian_pixies()).is_some(),
        "2 damage from the Bears kills a 2/1"
    );
}

/// The prevention names artifact *creatures*: Rod of Ruin is an artifact but
/// no creature, so its 1 damage kills the 2/1 Pixies.
#[test]
fn argothian_pixies_is_not_protected_from_a_noncreature_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                argothian_pixies(),
                rod_of_ruin(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pixies = on_battlefield(&engine, p0, argothian_pixies()).expect("seated");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rod_of_ruin(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![pixies],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, argothian_pixies()).is_some(),
        "Rod of Ruin's damage is not prevented"
    );
}
