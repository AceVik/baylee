//! `cards/creatures/mv_5/earth_elemental.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Earth Elemental — vanilla `{3}{R}{R}` 4/5 Elemental.
#[test]
fn earth_elemental_is_a_four_five_elemental_for_3rr() {
    let p0 = PlayerId::new(0);
    assert_eq!(
        cast_saying_nothing(earth_elemental(), mountain(), 5),
        Zone::Battlefield
    );
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[earth_elemental()])
        .start();
    keep_mulligans(&mut engine);
    let id = on_battlefield(&engine, p0, earth_elemental()).expect("seated");
    assert_eq!(pt(&engine, id), (4, 5));
}

/// Veteran Bodyguard — "all damage that would be dealt to you by
/// unblocked creatures is dealt to this creature instead": two unblocked
/// attackers whose combined power reaches the Bodyguard's 5 toughness are
/// both redirected onto it, and the marked total is lethal (CR 704.5g),
/// while its controller still takes nothing off their own life total.
#[test]
fn two_unblocked_attackers_whose_combined_damage_reaches_five_kill_the_untapped_bodyguard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gray_ogre(), earth_elemental()])
        .battlefield(1, &[veteran_bodyguard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    let elemental = on_battlefield(&engine, p0, earth_elemental()).expect("seated");
    let guard = on_battlefield(&engine, p1, veteran_bodyguard()).expect("seated");
    let before_life = engine.state().players[1].life;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (ogre, Defender::Player(p1)),
                    (elemental, Defender::Player(p1)),
                ],
            },
        )
        .expect("both attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks: both are unblocked");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().object(guard).map(|o| o.zone),
        Some(Zone::Graveyard),
        "2 plus 4 marked on a 5-toughness creature is lethal"
    );
    assert_eq!(
        engine.state().players[1].life,
        before_life,
        "still not a point off its controller"
    );
}

/// Veteran Bodyguard counter-check: "unblocked creatures" — a blocked
/// attacker's damage goes to its blocker only, and neither the Bodyguard
/// nor its controller sees it, while an unblocked attacker beside it is
/// still redirected onto the Bodyguard.
#[test]
fn a_blocked_attackers_damage_stays_on_its_blocker_while_an_unblocked_one_beside_it_is_redirected()
{
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gray_ogre(), earth_elemental()])
        .battlefield(1, &[veteran_bodyguard(), hurloon_minotaur()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    let elemental = on_battlefield(&engine, p0, earth_elemental()).expect("seated");
    let guard = on_battlefield(&engine, p1, veteran_bodyguard()).expect("seated");
    let minotaur = on_battlefield(&engine, p1, hurloon_minotaur()).expect("seated");
    let before_life = engine.state().players[1].life;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (ogre, Defender::Player(p1)),
                    (elemental, Defender::Player(p1)),
                ],
            },
        )
        .expect("both attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(minotaur, ogre)],
            },
        )
        .expect("the Minotaur blocks the Ogre; the Elemental is unblocked");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().object(minotaur).unwrap().damage,
        2,
        "the blocked Ogre's damage: on its blocker, not the Bodyguard"
    );
    assert_eq!(
        engine.state().object(guard).unwrap().damage,
        4,
        "the unblocked Elemental's damage: still redirected"
    );
    assert_eq!(
        engine.state().players[1].life,
        before_life,
        "neither attacker's damage reached the player"
    );
    assert_eq!(engine.state().per_turn.damage_dealt_to[1], 0);
}
