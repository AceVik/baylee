//! `cards/artifacts/equipment/mv_1/gorgon_s_head.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gorgon_s_head() -> CardIndex {
    card_index("190cdc23-fa05-475f-b853-471ca0f21b71")
}

/// Gorgon's Head — "Equipped creature has deathtouch." and "Equip {2}". The
/// equipped 1/1 Elf is blocked by a 3/3 Angel and kills it.
#[test]
fn gorgon_s_head_lets_a_one_one_kill_what_blocks_it() {
    let (elf, angel) = (llanowar_elves(), angel_of_mercy());
    let mut engine = rig(2, &[], &[gorgon_s_head(), elf], &[angel]);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let host = on_battlefield(&engine, p0, elf).expect("the Elf");
    let blocker = on_battlefield(&engine, p1, angel).expect("the Angel");
    assert!(!keywords(&engine, host).contains(KeywordSet::DEATHTOUCH));
    equip(&mut engine, gorgon_s_head(), 2, host);
    assert!(keywords(&engine, host).contains(KeywordSet::DEATHTOUCH));
    assert!(
        !keywords(&engine, blocker).contains(KeywordSet::DEATHTOUCH),
        "only the equipped creature"
    );

    let blocks = attack_and_collect_blocks(&mut engine, host, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == blocker && b.attackers.contains(&host)),
        "the Angel may block the Elf"
    );
    block_and_finish(&mut engine, vec![(blocker, host)]);
    assert!(
        in_graveyard(&engine, p1, angel).is_some(),
        "deathtouch: one damage is lethal to a 3/3"
    );
}
