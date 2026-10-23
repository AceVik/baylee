#[allow(clippy::wildcard_imports)]
use super::*;

fn bubbling_muck() -> CardIndex {
    card_index("3c7f2b29-9f42-41ab-a2d4-7a450fb0242d")
}

#[test]
fn bubbling_muck_causes_swamps_to_add_an_additional_black_mana_this_turn() {
    let p0 = PlayerId::new(0);
    let necro = card_index("d1a6aa5f-2ad6-4bc7-82f0-4262f66af730"); // Apprentice Necromancer {1}{B}
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[bubbling_muck(), necro])
        .battlefield(0, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let swamp1 = on_battlefield(&engine, p0, swamp()).unwrap();
    tap_mana_except(&mut engine, p0, swamp1);
    cast_with_floating(&mut engine, p0, bubbling_muck());
    pass_until(&mut engine, stack_is_empty);

    // One remaining Swamp now produces {B}{B}, enough to cast Apprentice Necromancer ({1}{B}).
    cast_from_hand(&mut engine, p0, necro);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, necro).is_some());
}
