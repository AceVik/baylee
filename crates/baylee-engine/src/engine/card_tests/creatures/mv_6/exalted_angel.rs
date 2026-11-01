//! `cards/creatures/mv_6/exalted_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn exalted_angel() -> CardIndex {
    card_index("2f6e9ab8-6951-4c5f-bcde-0e96e0fe87f2")
}

#[test]
fn exalted_angel_gains_life_equal_to_her_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[exalted_angel()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let angel = on_battlefield(&e, p0, exalted_angel()).expect("the Angel");
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(angel, Defender::Player(p1))],
        },
    )
    .expect("the Angel attacks");
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(e.state().players[1].life, 16);
    assert_eq!(
        e.state().players[0].life,
        24,
        "four damage gained four life"
    );
}

#[test]
fn exalted_angel_casts_face_down_and_turns_face_up_for_her_morph_cost() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 7])
        .hand(0, &[exalted_angel()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, exalted_angel());
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!(
            "normal and morph casts should be offered, got {:?}",
            e.pending()
        )
    };
    let morph = options
        .iter()
        .position(|option| matches!(option.kind, CastModeKind::Disguise))
        .expect("the shared face-down cast mode includes morph");
    e.apply(p0, PlayerAction::ChooseMode(morph))
        .expect("cast as a face-down 2/2 for {3}");
    pass_until(&mut e, stack_is_empty);
    let angel = on_battlefield(&e, p0, exalted_angel()).expect("the face-down creature");
    assert!(
        e.state()
            .object(angel)
            .expect("face down")
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, angel), (2, 2));

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: angel,
            ability_index: crate::choice::TURN_FACE_UP,
        },
    )
    .expect("the four floating mana pay morph {2}{W}{W}");
    assert!(
        !e.state()
            .object(angel)
            .expect("face up")
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, angel), (4, 5));
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}
