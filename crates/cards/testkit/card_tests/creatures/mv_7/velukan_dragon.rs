//! `cards/creatures/mv_7/velukan_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn velukan_dragon() -> CardIndex {
    card_index("667aef37-a47a-4593-a252-807c02416863")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

fn last_d6(engine: &Engine<RegistryLookup>, from: usize) -> i16 {
    engine.state().journal.entries()[from..]
        .iter()
        .find_map(|entry| match entry.event {
            GameEvent::DiceRolled {
                player: P0,
                sides: 6,
                result,
            } => Some(i16::try_from(result).unwrap()),
            _ => None,
        })
        .expect("expected a d6 roll by P0")
}

/// Velukan Dragon (667aef37-a47a-4593-a252-807c02416863) — `{5}{R}{R}` 5/5 Creature — Dragon:
/// - Flying
/// - Whenever this creature attacks or blocks, roll a six-sided die. This creature
///   gets +X/+0 until end of turn, where X is the result minus 1.
#[test]
fn velukan_dragon_rolls_d6_for_power_minus_one_on_both_attack_and_block() {
    let lands = vec![mountain(); 7];
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &lands)
        .battlefield(1, &[air_elemental()])
        .hand(0, &[velukan_dragon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, velukan_dragon());
    pass_until(&mut engine, stack_is_empty);

    let dragon = on_battlefield(&engine, P0, velukan_dragon()).expect("Velukan Dragon entered");
    let elem = on_battlefield(&engine, P1, air_elemental()).unwrap();
    assert_eq!(pt(&engine, dragon), (5, 5));
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));

    // P1's turn: Air Elemental (4/4) attacks; Velukan Dragon blocks -> rolls d6, gets +(roll-1)/+0.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elem, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let from_block = engine.state().journal.entries().len();
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(dragon, elem)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    let block_roll = last_d6(&engine, from_block);
    assert_eq!(pt(&engine, dragon), (5 + (block_roll - 1), 5));
    assert!(in_graveyard(&engine, P1, air_elemental()).is_some());

    // P0's turn: bonus reset to 5/5; attacking triggers another d6 roll.
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(pt(&engine, dragon), (5, 5));
    let from_attack = engine.state().journal.entries().len();
    let _ = attack_and_collect_blocks(&mut engine, dragon, P1);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    let attack_roll = last_d6(&engine, from_attack);
    assert_eq!(pt(&engine, dragon), (5 + (attack_roll - 1), 5));
    assert_eq!(
        engine.state().players[1].life,
        20 - i32::from(5 + (attack_roll - 1))
    );
}
