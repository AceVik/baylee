//! `cards/enchantments/mv_2/gossamer_chains.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn gossamer_chains() -> CardIndex {
    card_index("af3936ab-0156-438d-9fef-a924d2fcffc3")
}

/// `Gossamer Chains` ({W}{W}, Enchantment, `Coverage::Implemented`):
/// `Return this enchantment to its owner's hand: Prevent all combat damage that
/// would be dealt by target unblocked creature this turn.`
#[test]
fn gossamer_chains_returns_to_hand_to_prevent_combat_damage_from_unblocked_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[gossamer_chains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, gossamer_chains());
    pass_until(&mut engine, stack_is_empty);
    let _chains = on_battlefield(&engine, p0, gossamer_chains()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    // Advance to P1's combat and let Grizzly Bears attack unblocked.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    activate(&mut engine, p0, gossamer_chains(), 0);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(in_hand(&engine, p0, gossamer_chains()).is_some());
    assert_eq!(engine.state().players[0].life, 20);

    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[0].life,
        18,
        "the prevention expired at the end of the previous turn"
    );
}

/// "Prevent all combat damage that would be dealt by target unblocked creature this turn": of two
/// attackers, only the unblocked one is a target; its damage is prevented, while the blocked
/// attacker still kills its blocker.
#[test]
fn gossamer_chains_targets_only_the_unblocked_attacker_and_spares_the_other_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .battlefield(1, &[GRIZZLY_BEARS, gray_ogre()])
        .hand(0, &[gossamer_chains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    tap_mana_except(&mut engine, p0, elves);
    cast_with_floating(&mut engine, p0, gossamer_chains());
    pass_until(&mut engine, stack_is_empty);
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    assert!(
        !is_tapped(&engine, elves),
        "the blocker did not pay for the enchantment"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0)), (ogre, Defender::Player(p0))],
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
                blockers: vec![(elves, ogre)],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    activate(&mut engine, p0, gossamer_chains(), 0);
    let menu = aim_at(&mut engine, p0, bear);
    assert_eq!(menu, vec![bear], "only the unblocked attacker is a target");
    assert!(!menu.contains(&ogre), "a blocked attacker is not a target");
    assert!(
        in_hand(&engine, p0, gossamer_chains()).is_some(),
        "returning the enchantment is paid before its ability resolves"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(in_hand(&engine, p0, gossamer_chains()).is_some());
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the unblocked Bears dealt nothing"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the blocked Ogre's damage is not prevented"
    );
}

/// Prevention still assigns combat damage, so an effect that makes that
/// damage unpreventable defeats the Chains.
#[test]
fn gossamer_chains_cannot_prevent_unpreventable_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[gossamer_chains()])
        .battlefield(
            1,
            &[GRIZZLY_BEARS, baylee_core::generated::index::QUESTING_BEAST],
        )
        .start();
    keep_mulligans(&mut engine);
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    activate(&mut engine, p0, gossamer_chains(), 0);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(in_hand(&engine, p0, gossamer_chains()).is_some());
    assert_eq!(engine.state().players[0].life, 18);
}
