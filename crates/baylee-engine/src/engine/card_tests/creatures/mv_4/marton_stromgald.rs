//! `cards/creatures/mv_4/marton_stromgald.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn marton_stromgald() -> CardIndex {
    card_index("8364cf93-c323-46f4-854f-ae872e484438")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn board(opposing_creatures: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                marton_stromgald(),
                grizzly_bears(),
                grizzly_bears(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, opposing_creatures)
        .start();
    keep_mulligans(&mut engine);
    engine
}

fn attack(
    engine: &mut Engine<RegistryLookup>,
    player: PlayerId,
    defender: PlayerId,
    attackers: &[ObjectId],
) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&id| (id, Defender::Player(defender)))
                    .collect(),
            },
        )
        .unwrap();
}

/// Márton Stromgald (8364cf93-c323-46f4-854f-ae872e484438) — {2}{R}{R} 1/1 Legendary Creature — Human Knight:
/// - "Whenever Márton Stromgald attacks, other attacking creatures get +1/+1 until end of turn for each attacking creature other than Márton Stromgald."
/// - "Whenever Márton Stromgald blocks, other blocking creatures get +1/+1 until end of turn for each blocking creature other than Márton Stromgald."
#[test]
fn marton_stromgald_pumps_other_attacking_and_blocking_creatures_by_their_count_excluding_himself()
{
    // 1. Attack trigger: P0 controls Márton (1/1) and 3 Grizzly Bears (2/2).
    // P0 attacks with Márton and 2 of the Bears, leaving the 3rd Bear back.
    let mut engine = board(&[]);

    let marton = on_battlefield(&engine, P0, marton_stromgald()).unwrap();
    let bears = all_on_battlefield(&engine, P0, grizzly_bears());
    assert_eq!(bears.len(), 3);

    attack(&mut engine, P0, P1, &[marton, bears[0], bears[1]]);
    // Resolve Márton's attack trigger before combat damage.
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears[0]),
        (4, 4),
        "first attacking Bear gets +2/+2 for the 2 attacking creatures other than Márton"
    );
    assert_eq!(
        pt(&engine, bears[1]),
        (4, 4),
        "second attacking Bear gets +2/+2 for the 2 attacking creatures other than Márton"
    );
    assert_eq!(
        pt(&engine, marton),
        (1, 1),
        "Márton Stromgald himself is excluded from 'other attacking creatures'"
    );
    assert_eq!(
        pt(&engine, bears[2]),
        (2, 2),
        "the non-attacking Bear is unaffected"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    // 1 + 4 + 4 = 9 combat damage to P1.
    assert_eq!(engine.state().players[1].life, 11);

    // 2. Block trigger: On P1's turn, P1 attacks with 3 Grizzly Bears; P0 blocks with Márton and 2 Bears,
    // leaving P0's 3rd Bear non-blocking.
    let mut engine2 = board(&[grizzly_bears(), grizzly_bears(), grizzly_bears()]);
    reach_their_main_phase(&mut engine2, P1);

    let marton2 = on_battlefield(&engine2, P0, marton_stromgald()).unwrap();
    let p0_bears = all_on_battlefield(&engine2, P0, grizzly_bears());
    let p1_bears = all_on_battlefield(&engine2, P1, grizzly_bears());

    attack(&mut engine2, P1, P0, &p1_bears);
    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine2
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![
                    (marton2, p1_bears[0]),
                    (p0_bears[0], p1_bears[1]),
                    (p0_bears[1], p1_bears[2]),
                ],
            },
        )
        .unwrap();
    // Resolve Márton's block trigger before combat damage.
    pass_until(&mut engine2, stack_is_empty);

    assert_eq!(
        pt(&engine2, p0_bears[0]),
        (4, 4),
        "first blocking Bear gets +2/+2 for the 2 blocking creatures other than Márton"
    );
    assert_eq!(
        pt(&engine2, p0_bears[1]),
        (4, 4),
        "second blocking Bear gets +2/+2 for the 2 blocking creatures other than Márton"
    );
    assert_eq!(
        pt(&engine2, marton2),
        (1, 1),
        "Márton himself stays 1/1 when blocking"
    );
    assert_eq!(
        pt(&engine2, p0_bears[2]),
        (2, 2),
        "non-blocking Bear stays 2/2"
    );
    for &attacker in &p1_bears {
        assert_eq!(
            pt(&engine2, attacker),
            (2, 2),
            "the attackers are not blocking creatures: the block trigger pumps blockers only"
        );
    }
}
