//! `cards/artifacts/mv_5/coat_of_arms.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn coat_of_arms() -> CardIndex {
    card_index("5f7f133e-58ea-41ab-b1be-be4b400fac4c")
}

fn goblin_warchief() -> CardIndex {
    card_index("39882df0-c20f-469d-94ba-1617224e71a1")
}

fn akki_rockspeaker() -> CardIndex {
    card_index("01cd52f9-7fac-4396-bec7-d06bf683e011")
}

/// Coat of Arms (5f7f133e-58ea-41ab-b1be-be4b400fac4c) — {5} Artifact:
/// "Each creature gets +1/+1 for each other creature on the battlefield that
///  shares at least one creature type with it. (For example, if two Goblin
///  Warriors and a Goblin Shaman are on the battlefield, each gets +2/+2.)"
///
/// Mirrors the printed reminder text across both sides of the battlefield:
/// two Goblin Warriors (sharing TWO types: Goblin and Warrior — counted once!)
/// plus an opponent's Goblin Shaman each get +2/+2, an unrelated Elf Druid
/// gets +0/+0, and casting another Elf Druid updates both Elves to +1/+1 while
/// killing the Goblin Shaman drops both Goblin Warriors to +1/+1.
#[test]
fn coat_of_arms_counts_each_other_sharing_creature_once_across_both_sides() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                coat_of_arms(),
                goblin_warchief(),
                goblin_warchief(),
                mountain(),
                forest(),
            ],
        )
        .hand(0, &[lightning_bolt(), llanowar_elves()])
        .battlefield(1, &[akki_rockspeaker(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let warchiefs = all_on_battlefield(&engine, p0, goblin_warchief());
    assert_eq!(warchiefs.len(), 2);
    let shaman = on_battlefield(&engine, p1, akki_rockspeaker()).unwrap();
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    // Two 2/2 Goblin Warriors + one 1/1 Goblin Shaman across the table => each gets +2/+2.
    assert_eq!(
        pt(&engine, warchiefs[0]),
        (4, 4),
        "Goblin Warrior gets +2/+2 (other Goblin Warrior counts once, not twice)"
    );
    assert_eq!(pt(&engine, warchiefs[1]), (4, 4));
    assert_eq!(
        pt(&engine, shaman),
        (3, 3),
        "opponent's 1/1 Goblin Shaman also gets +2/+2"
    );
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "unrelated 1/1 Elf Druid gets +0/+0"
    );

    // Bolt opponent's 3/3 Goblin Shaman -> kills it; remaining two Goblin Warriors drop to +1/+1.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, shaman);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(shaman).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        pt(&engine, warchiefs[0]),
        (3, 3),
        "with only one other Goblin Warrior left, each gets +1/+1"
    );
    assert_eq!(pt(&engine, warchiefs[1]), (3, 3));

    // Cast our own Llanowar Elves (Elf Druid): both Elves (yours & opponent's) share
    // two types (Elf & Druid) and each gets +1/+1 (not +2/+2).
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert_eq!(pt(&engine, my_elf), (2, 2), "own Elf Druid gets +1/+1");
    assert_eq!(
        pt(&engine, their_elf),
        (2, 2),
        "opponent's Elf Druid gets +1/+1"
    );
    assert_eq!(
        pt(&engine, warchiefs[0]),
        (3, 3),
        "Goblins unaffected by Elves"
    );
}
