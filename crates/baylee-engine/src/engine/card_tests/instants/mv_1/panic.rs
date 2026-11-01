//! `cards/instants/mv_1/panic.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn panic_card() -> CardIndex {
    card_index("e98429a7-a83f-4b69-afad-9eb68b09e38e")
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Panic (e98429a7-a83f-4b69-afad-9eb68b09e38e) — {R} Instant:
/// - "Cast this spell only during combat before blockers are declared."
/// - "Target creature can't block this turn."
/// - "Draw a card at the beginning of the next turn's upkeep."
#[test]
fn panic_can_only_be_cast_during_combat_before_blockers_prevents_blocking_and_draws_next_upkeep() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[panic_card(), panic_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let first_panic = in_hand(&engine, P0, panic_card()).unwrap();
    let p0_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let p1_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let p1_ogre = on_battlefield(&engine, P1, gray_ogre()).unwrap();

    // 1. Outside combat (FirstMain), Panic is not castable even with mana floating.
    let first_mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == first_mtn);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&first_panic),
        "Panic cannot be cast outside combat"
    );

    // 2. Advance to DeclareAttackers: declare P0's Grizzly Bears as an attacker, then cast Panic
    // on P1's Grizzly Bears before blockers are declared.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(p0_bears, Defender::Player(P1))],
            },
        )
        .unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, panic_card());
    aim_at(&mut engine, P0, p1_bears);
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });

    // P1's Grizzly Bears cannot block P0's Grizzly Bears, but the untargeted Gray Ogre can.
    let Pending::ChooseBlockers {
        blockers: offered, ..
    } = engine.pending().clone()
    else {
        panic!("expected the block question, got {:?}", engine.pending());
    };
    assert!(
        !lk::may_block(&offered, p1_bears, p0_bears),
        "the targeted creature is not offered as a blocker"
    );
    assert!(
        lk::may_block(&offered, p1_ogre, p0_bears),
        "a creature Panic did not target can still block"
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(p1_bears, p0_bears)],
                },
            )
            .is_err(),
        "creature targeted by Panic cannot block this turn"
    );
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // 3. After blockers are declared (in DeclareBlockers priority), the second Panic in hand
    // is NOT castable even with {R} floating ("before blockers are declared").
    let second_panic = in_hand(&engine, P0, panic_card()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&second_panic),
        "Panic cannot be cast after blockers are declared"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 18);

    // 4. At the beginning of the next turn's upkeep (P1's upkeep, turn 2), P0 draws a card.
    let hand_before = hand_size(&engine, P0);
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        hand_size(&engine, P0),
        hand_before + 1,
        "Panic draws a card at the beginning of the next turn's upkeep"
    );
}
