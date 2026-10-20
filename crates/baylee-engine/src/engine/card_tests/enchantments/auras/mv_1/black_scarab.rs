//! `cards/enchantments/auras/mv_1/black_scarab.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn black_scarab() -> CardIndex {
    card_index("2774d8b3-dc5f-4281-b4c5-cb003fef0ab2")
}

/// Black Scarab (2774d8b3-dc5f-4281-b4c5-cb003fef0ab2) — {W} Enchantment — Aura:
/// - "Enchant creature"
/// - "Enchanted creature can't be blocked by black creatures."
/// - "Enchanted creature gets +2/+2 as long as an opponent controls a black
///   permanent."
#[test]
fn black_scarab_grants_plus_two_plus_two_when_opponent_controls_black_permanent_and_prevents_black_blockers()
 {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), llanowar_elves()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[black_scarab()])
        .hand(1, &[swamp(), festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_elf = on_battlefield(&engine, P0, llanowar_elves()).expect("my Elf seated");
    let their_ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their red Ogre seated");

    cast_from_hand(&mut engine, P0, black_scarab());
    aim_at(&mut engine, P0, my_elf);
    pass_until(&mut engine, stack_is_empty);

    // No opponent controls a black permanent yet -> no +2/+2 bonus (1/1).
    assert_eq!(
        pt(&engine, my_elf),
        (1, 1),
        "no black permanent controlled by an opponent yet"
    );

    // On P1's turn, P1 plays a Swamp and casts Festering Goblin (a black permanent).
    reach_their_main_phase(&mut engine, P1);
    play_land(&mut engine, P1, swamp());
    cast_from_hand(&mut engine, P1, festering_goblin());
    pass_until(&mut engine, stack_is_empty);

    let their_goblin =
        on_battlefield(&engine, P1, festering_goblin()).expect("black Goblin seated");

    // Now an opponent controls a black permanent -> enchanted Elf gets +2/+2 (3/3).
    assert_eq!(
        pt(&engine, my_elf),
        (3, 3),
        "+2/+2 while an opponent controls a black permanent"
    );

    // On P0's next turn, attack with enchanted Elf: black Festering Goblin cannot
    // block it, while red Gray Ogre can.
    assert!(walk_to_own_main(&mut engine, P0));
    let blocks = attack_and_collect_blocks(&mut engine, my_elf, P1);
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == their_goblin && b.attackers.contains(&my_elf)),
        "black creature cannot block creature enchanted by Black Scarab"
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == their_ogre && b.attackers.contains(&my_elf)),
        "non-black creature can still block"
    );
}
