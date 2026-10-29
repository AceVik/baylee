//! Damping Matrix — `{3}` Artifact (MRD).
//! Oracle: "Activated abilities of artifacts and creatures can't be activated unless they're mana abilities."

use super::*;

fn damping_matrix() -> CardIndex {
    card_index("10403a8c-2534-421e-8809-5fc019623452")
}

#[test]
fn damping_matrix_stops_non_mana_activated_abilities_of_artifacts_and_creatures_while_sparing_mana_abilities()
 {
    let p0 = PlayerId::new(0);
    let matrix = damping_matrix();
    let shard = card_index("cd8ccb10-a2c8-4a84-aa41-6ce6018f9bb7");
    let troll = lotleth_troll();
    let ring = sol_ring();
    let elf = llanowar_elves();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[shard, troll, ring, elf, island(), swamp(), forest()])
        .hand(0, &[matrix])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shard_id = on_battlefield(&engine, p0, shard).unwrap();
    let troll_id = on_battlefield(&engine, p0, troll).unwrap();
    let ring_id = on_battlefield(&engine, p0, ring).unwrap();
    let elf_id = on_battlefield(&engine, p0, elf).unwrap();

    // Before Damping Matrix enters, non-mana abilities on Crystal Shard and Lotleth Troll are offered.
    assert!(ability_offered(&engine, shard_id, 1));
    assert!(ability_offered(&engine, troll_id, 1));

    // Cast Damping Matrix using the 3 lands (leaving Sol Ring and Llanowar Elves untapped).
    tap_mana_where(&mut engine, p0, |id| id != ring_id && id != elf_id);
    cast_with_floating(&mut engine, p0, matrix);
    pass_until(&mut engine, stack_is_empty);

    // With Damping Matrix on the battlefield, non-mana activated abilities of artifacts and creatures cannot be activated.
    assert!(!ability_offered(&engine, shard_id, 0));
    assert!(!ability_offered(&engine, shard_id, 1));
    assert!(!ability_offered(&engine, troll_id, 1));

    // Mana abilities of artifacts (Sol Ring) and creatures (Llanowar Elves) can still be activated.
    tap_mana_where(&mut engine, p0, |id| id == ring_id);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2
    );
}
