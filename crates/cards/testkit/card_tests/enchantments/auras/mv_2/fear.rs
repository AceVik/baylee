//! `cards/enchantments/auras/mv_2/fear.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fear: "Enchanted creature has fear." A black creature may still block
/// it; a non-black, non-artifact creature may not.
#[test]
fn fear_cannot_be_blocked_except_by_artifact_or_black_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let fear = fear();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), aurochs()])
        .battlefield(1, &[llanowar_elves(), festering_goblin()])
        .hand(0, &[fear])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker = on_battlefield(&engine, p0, aurochs()).expect("attacker seated");

    cast_from_hand(&mut engine, p0, fear);
    aim_at(&mut engine, p0, attacker);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, attacker).contains(KeywordSet::FEAR));

    let blockers = attack_and_collect_blocks(&mut engine, attacker, p1);
    let elf_id = on_battlefield(&engine, p1, llanowar_elves()).expect("green elf seated");
    let goblin_id = on_battlefield(&engine, p1, festering_goblin()).expect("black goblin seated");
    let elf_option = blockers.iter().find(|b| b.blocker == elf_id);
    assert!(
        elf_option.is_none_or(|b| !b.attackers.contains(&attacker)),
        "a green, non-artifact creature may not block it: {blockers:?}"
    );
    let goblin_option = blockers.iter().find(|b| b.blocker == goblin_id);
    assert!(
        goblin_option.is_some_and(|b| b.attackers.contains(&attacker)),
        "a black creature may still block it: {blockers:?}"
    );
}
