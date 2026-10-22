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
}
