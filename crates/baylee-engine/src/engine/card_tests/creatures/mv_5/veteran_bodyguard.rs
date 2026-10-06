//! `cards/creatures/mv_5/veteran_bodyguard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Veteran Bodyguard — "As long as this creature is untapped, all damage
/// that would be dealt to you by unblocked creatures is dealt to this
/// creature instead." An unblocked Gray Ogre's 2 combat damage is marked
/// on the untapped Bodyguard instead: its controller's life and the
/// turn's damage tally are untouched, and the journal names the Ogre as
/// the source, the Bodyguard as the target, and the damage as combat
/// (CR 614.9).
#[test]
fn an_unblocked_attackers_combat_damage_marks_the_untapped_bodyguard_and_not_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gray_ogre()])
        .battlefield(1, &[veteran_bodyguard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    let guard = on_battlefield(&engine, p1, veteran_bodyguard()).expect("seated");
    assert!(!is_tapped(&engine, guard), "untapped to start");
    let before_life = engine.state().players[1].life;
    let before_journal = engine.journal().entries().len();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p1))],
            },
        )
        .expect("the Ogre attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks: the Ogre is unblocked");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().object(guard).unwrap().damage,
        2,
        "the Ogre's 2 power, marked on the Bodyguard instead of its controller"
    );
    assert_eq!(
        engine.state().players[1].life,
        before_life,
        "not a point off its controller"
    );
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[1],
        0,
        "the player was never dealt damage this turn"
    );
    assert_eq!(
        damage_events(&engine, before_journal),
        vec![(ogre, crate::event::DamageTarget::Object(guard), 2, true)],
        "one event of combat damage, from the Ogre to the Bodyguard"
    );
}

/// Veteran Bodyguard counter-check: a Lightning Bolt at the Bodyguard's
/// controller is not damage "by unblocked creatures" — a spell is no
/// creature — so nothing redirects it: the player takes the full 3.
#[test]
fn a_lightning_bolt_at_the_bodyguards_controller_is_not_from_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[veteran_bodyguard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p1, veteran_bodyguard()).expect("seated");
    let before_life = engine.state().players[1].life;

    cast_from_hand(&mut engine, p0, lightning_bolt());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("the Bolt targets, got {:?}", engine.pending())
    };
    assert!(player_options.contains(&p1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opposing player is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        before_life - 3,
        "a spell, not a creature: the Bodyguard does not intercept it"
    );
    assert_eq!(
        engine.state().object(guard).unwrap().damage,
        0,
        "nothing marked on the Bodyguard"
    );
}

/// Veteran Bodyguard — "As long as this creature is untapped, all damage
/// that would be dealt to you by unblocked creatures is dealt to this
/// creature instead." Its own controller attacks with it, so it stays
/// tapped through the opponent's following untap step (only the active
/// player's permanents untap, CR 502.3), and the opponent's unblocked
/// attacker deals its damage straight to the player instead of to the
/// now-tapped Bodyguard. Paired on the same board: once the Bodyguard's
/// own next untap step frees it again, the same attacker's damage is
/// redirected exactly as before the first attack — the tapped half is
/// not a coincidence of this board, and the static is not gone for good.
#[test]
#[allow(clippy::too_many_lines)] // one board, carried through four turns to pair tapped and untapped
fn the_bodyguard_stays_tapped_from_its_own_attack_and_lets_the_next_unblocked_attack_through() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[veteran_bodyguard()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p0, veteran_bodyguard()).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");

    // Turn 1: the Bodyguard's own controller attacks with it.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(guard, Defender::Player(p1))],
            },
        )
        .expect("the Bodyguard attacks");
    assert!(
        is_tapped(&engine, guard),
        "attacking taps it (no vigilance)"
    );

    // Turn 2: p1's untap step untaps only p1's permanents.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    assert!(
        is_tapped(&engine, guard),
        "still tapped: p1's untap step is not the Bodyguard's own"
    );
    let before_life = engine.state().players[0].life;
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p0))],
            },
        )
        .expect("the Ogre attacks p0");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("a tapped Bodyguard cannot block anyway");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[0].life,
        before_life - 2,
        "the tapped Bodyguard no longer redirects: the Ogre's damage reached the player"
    );
    assert_eq!(
        engine.state().object(guard).unwrap().damage,
        0,
        "nothing marked on the tapped Bodyguard"
    );
    assert_eq!(engine.state().per_turn.damage_dealt_to[0], 2);

    // Turn 3: p0's own untap step frees the Bodyguard again. It holds it
    // back this time, so it stays untapped for what follows.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    assert!(
        !is_tapped(&engine, guard),
        "p0's own untap step untaps the Bodyguard again"
    );
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .expect("p0 holds it back this time");

    // Turn 4: the same Ogre attacks again, unblocked. The pairing: the
    // same board, the same attacker, only the Bodyguard's tapped status
    // changed, and the redirection is back.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p0))],
            },
        )
        .expect("the Ogre attacks p0 again");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[0].life,
        before_life - 2,
        "untapped again: the redirection resumed, so no further life was lost"
    );
    assert_eq!(
        engine.state().object(guard).unwrap().damage,
        2,
        "the Ogre's damage, marked on the Bodyguard now that it is untapped"
    );
}

/// Veteran Bodyguard counter-check, with a real deathtouch attacker: an
/// unblocked Baleful Strix's damage is still redirected onto the untapped
/// Bodyguard (CR 614.9), and the source's own deathtouch travels with the
/// damage it deals wherever it lands (CR 702.2b + 704.5h: any nonzero
/// damage from a deathtouch source is lethal) — one marked point is
/// lethal regardless of the Bodyguard's 5 toughness.
#[test]
fn an_unblocked_deathtouchers_redirected_damage_still_destroys_the_untapped_bodyguard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[baleful_strix()])
        .battlefield(1, &[veteran_bodyguard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let strix = on_battlefield(&engine, p0, baleful_strix()).expect("seated");
    let guard = on_battlefield(&engine, p1, veteran_bodyguard()).expect("seated");
    assert!(keywords(&engine, strix).contains(KeywordSet::DEATHTOUCH));
    assert!(!is_tapped(&engine, guard), "untapped to start");
    let before_life = engine.state().players[1].life;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(strix, Defender::Player(p1))],
            },
        )
        .expect("the Strix attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("flying: the Bodyguard has neither flying nor reach");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        before_life,
        "not a point off its controller: the static caught it first"
    );
    assert_eq!(
        engine.state().object(guard).map(|o| o.zone),
        Some(Zone::Graveyard),
        "one point of deathtouch damage is lethal, even redirected onto it"
    );
}
