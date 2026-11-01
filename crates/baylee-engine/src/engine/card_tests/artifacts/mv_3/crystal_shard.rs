//! Crystal Shard — `{3}` Artifact (MRD).
//! Oracle: "{3}, {T} or {U}, {T}: Return target creature to its owner's hand unless its controller pays {1}."

use super::*;

fn crystal_shard() -> CardIndex {
    card_index("cd8ccb10-a2c8-4a84-aa41-6ce6018f9bb7")
}

#[test]
fn crystal_shard_bounces_target_creature_unless_controller_pays_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let shard = crystal_shard();
    let elf = llanowar_elves();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[shard, island(), island(), island(), island(), elf])
        .battlefield(1, &[elf, forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shard_id = on_battlefield(&engine, p0, shard).unwrap();
    let my_elf = on_battlefield(&engine, p0, elf).unwrap();
    let their_elf = on_battlefield(&engine, p1, elf).unwrap();
    let islands = all_on_battlefield(&engine, p0, island());

    // Activate {U}, {T} (ability 1) targeting our own Elf and decline to pay {1}.
    tap_mana_where(&mut engine, p0, |id| id == islands[0]);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: shard_id,
                ability_index: 1,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, my_elf);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, elf).is_none());
    assert!(in_hand(&engine, p0, elf).is_some());
    assert!(on_battlefield(&engine, p1, elf).is_some());
    let _ = their_elf;
}
