//! Glissa Sunseeker — `{2}{G}{G}` 3/2 Legendary Creature — Elf Warrior (MRD).
//! Oracle: "First strike. {T}: Destroy target artifact if its mana value is equal to the amount of unspent mana you have."

use super::*;

fn glissa_sunseeker() -> CardIndex {
    card_index("bf4b9330-72df-426f-820c-77e37f7e9130")
}

#[test]
fn glissa_sunseeker_destroys_target_artifact_only_when_unspent_mana_equals_mana_value() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let glissa = glissa_sunseeker();
    let ring = sol_ring();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[glissa, forest()])
        .battlefield(1, &[ring])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let glissa_id = on_battlefield(&engine, p0, glissa).unwrap();
    let ring_id = on_battlefield(&engine, p1, ring).unwrap();
    assert!(keywords(&engine, glissa_id).contains(KeywordSet::FIRST_STRIKE));

    // Tap 1 Forest for 1 unspent mana, matching Sol Ring's mana value (1).
    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, glissa, 0);
    aim_at(&mut engine, p0, ring_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, ring).is_none());
    assert!(in_graveyard(&engine, p1, ring).is_some());
}
