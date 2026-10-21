//! `cards/enchantments/auras/mv_1/red_scarab.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn red_scarab() -> CardIndex {
    card_index("380b6a48-c364-4939-be0b-df542c63a5ff")
}

/// Red Scarab (380b6a48-c364-4939-be0b-df542c63a5ff) — {W} Enchantment — Aura:
/// - "Enchant creature"
/// - "Enchanted creature can't be blocked by red creatures."
/// - "Enchanted creature gets +2/+2 as long as an opponent controls a red permanent."
#[test]
fn red_scarab_grants_plus_two_plus_two_when_opponent_controls_red_permanent_and_prevents_red_blockers()
 {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), llanowar_elves(), gray_ogre()])
        .battlefield(1, &[grizzly_bears(), mountain(), mountain()])
        .hand(0, &[red_scarab()])
        .hand(1, &[mountain(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_elf = on_battlefield(&engine, P0, llanowar_elves()).unwrap();
    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, red_scarab());
    aim_at(&mut engine, P0, my_elf);
    pass_until(&mut engine, stack_is_empty);

    // 1. Only P0 controls a red permanent (Gray Ogre), while P1 controls only Mountains and a green creature -> no +2/+2 bonus (1/1).
    assert_eq!(
        pt(&engine, my_elf),
        (1, 1),
        "P0's own red Gray Ogre does not satisfy 'as long as an opponent controls a red permanent'"
    );

    // 2. On P1's turn, P1 plays a Mountain and casts Gray Ogre (an opponent's red permanent) -> enchanted Elf becomes 3/3.
    reach_their_main_phase(&mut engine, P1);
    play_land(&mut engine, P1, mountain());
    cast_from_hand(&mut engine, P1, gray_ogre());
    pass_until(&mut engine, stack_is_empty);

    let p1_ogre = on_battlefield(&engine, P1, gray_ogre()).unwrap();
    assert_eq!(
        pt(&engine, my_elf),
        (3, 3),
        "enchanted creature gets +2/+2 while an opponent controls a red permanent"
    );

    // 3. On P0's next turn, attack with enchanted Elf: P1's red Gray Ogre cannot block it, while green Grizzly Bears can.
    assert!(walk_to_own_main(&mut engine, P0));
    let blocks = attack_and_collect_blocks(&mut engine, my_elf, P1);
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == p1_ogre && b.attackers.contains(&my_elf)),
        "red creature cannot block creature enchanted by Red Scarab"
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == their_bears && b.attackers.contains(&my_elf)),
        "non-red creature can block creature enchanted by Red Scarab"
    );
}
