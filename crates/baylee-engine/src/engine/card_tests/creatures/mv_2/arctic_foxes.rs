//! `cards/creatures/mv_2/arctic_foxes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn arctic_foxes() -> CardIndex {
    card_index("898f0824-cf4c-4e34-8407-9a32fdae6dcb")
}

/// Whether the blockers question offers `blocker` against `attacker`.
fn may_block(blocks: &[crate::choice::BlockOption], blocker: ObjectId, attacker: ObjectId) -> bool {
    blocks
        .iter()
        .any(|b| b.blocker == blocker && b.attackers.contains(&attacker))
}

/// Arctic Foxes (898f0824-cf4c-4e34-8407-9a32fdae6dcb) — {1}{W} 1/1 Creature — Fox:
/// - "This creature can't be blocked by creatures with power 2 or greater as
///   long as defending player controls a snow land."
///
/// P1 controls a snow land: P1's 2/2 Gray Ogre may not block the Foxes, and
/// P1's 1/1 Llanowar Elves still may.
#[test]
fn arctic_foxes_cannot_be_blocked_by_power_two_or_greater_while_the_defender_controls_a_snow_land()
{
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[arctic_foxes()])
        .battlefield(
            1,
            &[index::SNOW_COVERED_FOREST, gray_ogre(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    let foxes = on_battlefield(&engine, P0, arctic_foxes()).expect("Arctic Foxes seated");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their 2/2 Ogre");
    let elves = on_battlefield(&engine, P1, llanowar_elves()).expect("their 1/1 Elves");

    let blocks = attack_and_collect_blocks(&mut engine, foxes, P1);
    assert!(
        !may_block(&blocks, ogre, foxes),
        "power 2 may not block while P1 controls a snow land: {blocks:?}"
    );
    assert!(
        may_block(&blocks, elves, foxes),
        "power 1 still may: {blocks:?}"
    );
}

/// "As long as defending player controls a snow land": the snow land here is
/// P0's, and the defending player controls none, so P1's 2/2 may block.
#[test]
fn arctic_foxes_can_be_blocked_by_power_two_while_the_defender_controls_no_snow_land() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[arctic_foxes(), index::SNOW_COVERED_PLAINS])
        .battlefield(1, &[forest(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let foxes = on_battlefield(&engine, P0, arctic_foxes()).expect("Arctic Foxes seated");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their 2/2 Ogre");

    let blocks = attack_and_collect_blocks(&mut engine, foxes, P1);
    assert!(
        may_block(&blocks, ogre, foxes),
        "no snow land on the defending player's side: {blocks:?}"
    );
}
