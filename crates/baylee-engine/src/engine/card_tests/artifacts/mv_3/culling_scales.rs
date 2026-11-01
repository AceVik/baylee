//! Culling Scales — `{3}` Artifact (MRD).
//! Oracle: "At the beginning of your upkeep, destroy target nonland permanent with the lowest mana value."

use super::*;

fn culling_scales() -> CardIndex {
    card_index("ec4f2266-3e8e-4194-8f06-d95ebc543661")
}

#[test]
fn culling_scales_targets_and_destroys_only_lowest_mana_value_nonland_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let scales = culling_scales();
    let elf = llanowar_elves();
    let hunter = fangren_hunter();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scales, hunter, forest()])
        .battlefield(1, &[elf, forest()])
        .start();
    keep_mulligans(&mut engine);

    let scales_id = on_battlefield(&engine, p0, scales).unwrap();
    let hunter_id = on_battlefield(&engine, p0, hunter).unwrap();
    let their_elf = on_battlefield(&engine, p1, elf).unwrap();

    // On turn 1 upkeep, Culling Scales triggers and asks p0 to target the nonland permanent with lowest MV.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut engine, p0, their_elf);
    assert_eq!(
        options,
        vec![their_elf],
        "only the MV-1 Llanowar Elves has the lowest nonland mana value"
    );
    assert!(!options.contains(&scales_id) && !options.contains(&hunter_id));
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, elf).is_none());
    assert!(in_graveyard(&engine, p1, elf).is_some());
}
