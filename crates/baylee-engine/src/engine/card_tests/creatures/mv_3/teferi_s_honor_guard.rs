//! `cards/creatures/mv_3/teferi_s_honor_guard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn teferi_s_honor_guard() -> CardIndex {
    card_index("8a3a8a02-b7da-4648-a772-c9efb7c332da")
}

/// P0 casts their Honor Guard, waits for their next turn, attacks with it and
/// has `blocker` block. Returns the Guard once the blocks are declared.
fn cast_attack_and_block(engine: &mut Engine<RegistryLookup>, blocker: ObjectId) -> ObjectId {
    cast_from_hand(engine, P0, teferi_s_honor_guard());
    pass_until(engine, stack_is_empty);
    let guard = on_battlefield(engine, P0, teferi_s_honor_guard()).unwrap();

    reach_their_main_phase(engine, P1);
    reach_their_main_phase(engine, P0);
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(guard, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, guard)],
            },
        )
        .unwrap();
    guard
}

/// `Teferi's Honor Guard` ({2}{W}, 2/2 Creature — Human Knight,
/// `Coverage::Implemented`) has flanking and `{U}{U}: This creature phases out.`
#[test]
fn teferi_s_honor_guard_has_flanking_and_phases_out() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[teferi_s_honor_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let blocker = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let guard = cast_attack_and_block(&mut engine, blocker);
    // Resolve the flanking trigger first, then activate {U}{U} to phase out
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
            && stack_is_empty(e)
    });
    // Exactly one -1/-1: a 2/2 Bears becomes 1/1.
    assert_eq!(pt(&engine, blocker), (1, 1));
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, teferi_s_honor_guard(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(guard)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}

/// "A creature without flanking": a blocker with flanking is not shrunk.
#[test]
fn teferi_s_honor_guard_does_not_shrink_a_blocker_with_flanking() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[teferi_s_honor_guard()])
        .hand(0, &[teferi_s_honor_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let blocker = on_battlefield(&engine, P1, teferi_s_honor_guard()).unwrap();
    cast_attack_and_block(&mut engine, blocker);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, blocker),
        (2, 2),
        "a flanking blocker keeps its size"
    );
}

/// "It phases in before you untap during your next untap step": it stays
/// phased out through the opponent's turn and is back on its controller's next
/// turn.
#[test]
fn teferi_s_honor_guard_phases_in_at_its_controllers_next_untap() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), island(), island()])
        .hand(0, &[teferi_s_honor_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, teferi_s_honor_guard());
    pass_until(&mut engine, stack_is_empty);
    let guard = on_battlefield(&engine, P0, teferi_s_honor_guard()).unwrap();
    // The Guard was cast off the five lands; the next turn's untap gives {U}{U}.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, teferi_s_honor_guard(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(guard)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        engine
            .state()
            .object(guard)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "still phased out through the opponent's turn"
    );
    reach_their_main_phase(&mut engine, P0);
    assert!(
        !engine
            .state()
            .object(guard)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "phased in before its controller untaps"
    );
}
