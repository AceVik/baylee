//! `cards/enchantments/auras/mv_1/blue_scarab.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn blue_scarab() -> CardIndex {
    card_index("47c12d2f-b458-4848-9a01-0fc37ef3d942")
}

fn merfolk_of_the_pearl_trident() -> CardIndex {
    card_index("218d9277-c179-4de3-9c7f-79b5a6d4fa38")
}

/// Blue Scarab (47c12d2f-b458-4848-9a01-0fc37ef3d942) — {W} Enchantment — Aura:
/// - "Enchant creature"
/// - "Enchanted creature can't be blocked by blue creatures."
/// - "Enchanted creature gets +2/+2 as long as an opponent controls a blue
///   permanent."
///
/// P0's own blue Merfolk is there from the start and earns nothing: the
/// bonus waits for an opponent's blue permanent.
#[test]
fn blue_scarab_grants_plus_two_plus_two_when_opponent_controls_blue_permanent_and_prevents_blue_blockers()
 {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), llanowar_elves(), merfolk_of_the_pearl_trident()],
        )
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[blue_scarab()])
        .hand(1, &[island(), merfolk_of_the_pearl_trident()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_elf = on_battlefield(&engine, P0, llanowar_elves()).expect("my Elf seated");
    let their_ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their red Ogre seated");

    cast_from_hand(&mut engine, P0, blue_scarab());
    aim_at(&mut engine, P0, my_elf);
    pass_until(&mut engine, stack_is_empty);

    // Only P0 controls a blue permanent -> no +2/+2 bonus (1/1).
    assert_eq!(
        pt(&engine, my_elf),
        (1, 1),
        "P0's own blue Merfolk is not an opponent's blue permanent"
    );

    // On P1's turn, P1 plays an Island and casts Merfolk of the Pearl Trident (a blue permanent).
    reach_their_main_phase(&mut engine, P1);
    play_land(&mut engine, P1, island());
    cast_from_hand(&mut engine, P1, merfolk_of_the_pearl_trident());
    pass_until(&mut engine, stack_is_empty);

    let their_merfolk =
        on_battlefield(&engine, P1, merfolk_of_the_pearl_trident()).expect("blue Merfolk seated");

    // Now an opponent controls a blue permanent -> enchanted Elf gets +2/+2 (3/3).
    assert_eq!(
        pt(&engine, my_elf),
        (3, 3),
        "+2/+2 while an opponent controls a blue permanent"
    );

    // On P0's next turn, attack with enchanted Elf: blue Merfolk cannot block it,
    // while red Gray Ogre can.
    assert!(walk_to_own_main(&mut engine, P0));
    let blocks = attack_and_collect_blocks(&mut engine, my_elf, P1);
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == their_merfolk && b.attackers.contains(&my_elf)),
        "blue creature cannot block creature enchanted by Blue Scarab"
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == their_ogre && b.attackers.contains(&my_elf)),
        "non-blue creature can still block"
    );
}
