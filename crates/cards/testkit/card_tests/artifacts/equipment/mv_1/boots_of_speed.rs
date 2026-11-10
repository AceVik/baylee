//! `cards/artifacts/equipment/mv_1/boots_of_speed.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn boots_of_speed() -> CardIndex {
    card_index("6f6601bc-cb12-4054-9027-e4a0ebdae1a3")
}

/// Boots of Speed — "Equipped creature gets +1/+0 and has haste." and
/// "Equip {1}". Two Elves are cast this turn; the booted one may attack and
/// hits for 2, the other, summoning sick, is not offered.
#[test]
fn boots_of_speed_let_a_fresh_creature_attack() {
    let elf = llanowar_elves();
    let mut engine = rig(4, &[elf, elf], &[boots_of_speed()], &[]);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for _ in 0..2 {
        float(&mut engine, 1);
        cast_with_floating(&mut engine, p0, elf);
        pass_until(&mut engine, stack_is_empty);
    }
    let elves = all_on_battlefield(&engine, p0, elf);
    assert_eq!(elves.len(), 2);
    let (shod, bare) = (elves[0], elves[1]);
    assert!(!keywords(&engine, shod).contains(KeywordSet::HASTE));

    equip(&mut engine, boots_of_speed(), 1, shod);
    assert_eq!(pt(&engine, shod), (2, 1), "+1/+0");
    assert!(keywords(&engine, shod).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, bare).contains(KeywordSet::HASTE));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(attackers.contains(&shod), "haste: it may attack");
    assert!(!attackers.contains(&bare), "summoning sick: it may not");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shod, Defender::Player(p1))],
            },
        )
        .expect("the hasty Elf attacks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 18, "2 damage, +1 power");
}
