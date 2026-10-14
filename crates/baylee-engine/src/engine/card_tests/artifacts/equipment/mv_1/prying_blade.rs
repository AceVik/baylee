//! `cards/artifacts/equipment/mv_1/prying_blade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn prying_blade() -> CardIndex {
    card_index("5fe86083-1009-4de2-b186-f817ad1cc4e4")
}

/// Prying Blade — "Equipped creature gets +1/+0. Whenever equipped creature
/// deals combat damage to a player, create a Treasure token." and "Equip
/// {2}". An unblocked equipped Elf hits for 2 and a Treasure appears.
#[test]
fn prying_blade_makes_a_treasure_on_combat_damage_to_a_player() {
    let elf = llanowar_elves();
    let mut engine = rig(2, &[], &[prying_blade(), elf], &[]);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let host = on_battlefield(&engine, p0, elf).expect("the Elf");
    equip(&mut engine, prying_blade(), 2, host);
    assert_eq!(pt(&engine, host), (2, 1));
    assert!(tokens_of(&engine, p0).is_empty());

    let _ = attack_and_collect_blocks(&mut engine, host, p1);
    block_and_finish(&mut engine, vec![]);
    assert_eq!(engine.state().players[1].life, 18);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Treasure");
    let types = engine
        .state()
        .object(tokens[0])
        .expect("the token")
        .characteristics()
        .types;
    assert!(
        types.contains(TypeSet::ARTIFACT),
        "a Treasure is an artifact"
    );
}

/// "To a player": a blocked Elf deals its damage to the blocker and the
/// Blade makes nothing.
#[test]
fn prying_blade_makes_nothing_when_blocked() {
    let elf = llanowar_elves();
    let mut engine = rig(2, &[], &[prying_blade(), elf], &[elf]);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let host = on_battlefield(&engine, p0, elf).expect("the Elf");
    let blocker = on_battlefield(&engine, p1, elf).expect("their Elf");
    equip(&mut engine, prying_blade(), 2, host);
    let _ = attack_and_collect_blocks(&mut engine, host, p1);
    block_and_finish(&mut engine, vec![(blocker, host)]);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(tokens_of(&engine, p0).is_empty(), "no player was hit");
}
