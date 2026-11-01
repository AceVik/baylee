//! `cards/instants/mv_2/constant_mists.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn constant_mists() -> CardIndex {
    card_index("c850a29e-dc40-4ab6-89f1-d501a1a350d1")
}

/// Constant Mists (`c850a29e-dc40-4ab6-89f1-d501a1a350d1`) — {1}{G} Instant (Partial):
/// "Prevent all combat damage that would be dealt this turn."
#[test]
fn constant_mists_prevents_all_combat_damage() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), index::GRIZZLY_BEARS])
        .hand(0, &[constant_mists()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, constant_mists());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, constant_mists()).is_some(),
        "base spell does not get free buyback"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn fog_does_not_stop_noncombat_damage_and_ends_with_the_turn() {
    let mut e = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[constant_mists(), lightning_bolt()],
        &[],
    );
    let bear = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    lk::float(&mut e, P0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, P0, constant_mists());
    lk::settle(&mut e);
    // Pay the green spell before adding red: its generic cost may otherwise spend red.
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, P0, lightning_bolt(), &[], &[P1]);
    assert_eq!(lk::life(&e, P1), 17, "noncombat damage is not prevented");
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    lk::attackers_offered(&mut e);
    lk::declare(&mut e, &[bear]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(
        lk::life(&e, P1),
        15,
        "next turn combat damage is not prevented"
    );
}
