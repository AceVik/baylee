//! Incite War entwined grants first strike and forces the target's creatures to attack.

use super::*;

fn incite_war() -> CardIndex {
    card_index("8a640369-59ab-4506-b473-1f804721414c")
}

#[test]
fn incite_war_entwined_grants_first_strike_and_forces_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                gray_ogre(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[incite_war()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut e, p0, incite_war());
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("expected modes")
    };
    let mode = options
        .iter()
        .position(|option| option.kind == CastModeKind::Modes(3))
        .expect("entwine is offered");
    e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    match e.pending().clone() {
        Pending::ChooseTargets { .. } => e
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap(),
        Pending::ChoosePlayer { .. } => e.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap(),
        other => panic!("expected the target player, got {other:?}"),
    }
    pass_until(&mut e, stack_is_empty);
    let ours = on_battlefield(&e, p0, gray_ogre()).unwrap();
    assert!(keywords(&e, ours).contains(KeywordSet::FIRST_STRIKE));

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let theirs = on_battlefield(&e, p1, gray_ogre()).unwrap();
    assert!(
        e.apply(p1, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "an able creature may not stay home"
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(theirs, Defender::Player(p0))],
        },
    )
    .unwrap();
}

/// "Creatures you control gain first strike": the first-strike mode alone
/// reaches our Ogre and not the opponent's.
#[test]
fn incite_war_first_strike_mode_skips_the_opponent_s_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[gray_ogre(), mountain(), mountain(), mountain()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[incite_war()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, incite_war());
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("expected modes")
    };
    // The second printed mode alone, however the offer spells a single mode.
    let mode = options
        .iter()
        .position(|option| {
            option.kind == CastModeKind::Mode(1) || option.kind == CastModeKind::Modes(0b10)
        })
        .expect("the first-strike mode is offered on its own");
    e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    pass_until(&mut e, stack_is_empty);
    let ours = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let theirs = on_battlefield(&e, p1, gray_ogre()).unwrap();
    assert!(keywords(&e, ours).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&e, theirs).contains(KeywordSet::FIRST_STRIKE));
}
