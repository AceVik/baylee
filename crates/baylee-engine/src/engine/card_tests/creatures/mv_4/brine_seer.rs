#[allow(clippy::wildcard_imports)]
use super::*;

fn brine_seer() -> CardIndex {
    card_index("64c88c94-3a9d-4fd4-b58c-4d2167c2b8d5")
}

#[test]
fn brine_seer_counters_target_spell_unless_controller_pays_for_blue_cards_in_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[gray_ogre()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(1, &[counterspell()])
        .battlefield(1, &[brine_seer(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, gray_ogre());
    let spell_id = on_stack(&engine, gray_ogre()).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, brine_seer(), 0);
    aim_at(&mut engine, p1, spell_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if let Pending::YesNo { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::YesNo(false)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p0, gray_ogre()).is_some());
}
