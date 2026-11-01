//! Arc-Slogger — `{3}{R}{R}` 4/5 Creature — Beast (MRD).
//! Oracle: "{R}, Exile the top ten cards of your library: This creature deals 2 damage to any target."

use super::*;

fn arc_slogger() -> CardIndex {
    card_index("a28fc506-a98c-4b4d-a0f4-7d971489ccbb")
}

#[test]
fn arc_slogger_exiles_top_ten_cards_of_library_to_deal_two_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let slogger = arc_slogger();
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[slogger, mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib_before = library_size(&engine, p0);
    let exile_before = engine.state().zones.list(ZoneLocation::Exile(p0)).len();

    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, slogger, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), lib_before - 10);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        exile_before + 10
    );
    assert_eq!(engine.state().players[1].life, 18);
}
