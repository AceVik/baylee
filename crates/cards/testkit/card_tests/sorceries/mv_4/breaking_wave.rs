//! `cards/sorceries/mv_4/breaking_wave.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "f455b731-6ff9-4acb-9447-f524faa49b27"
fn breaking_wave() -> CardIndex {
    card_index("f455b731-6ff9-4acb-9447-f524faa49b27")
}

/// The paid flash route works on the opponent's turn. Both sides' tapped and
/// untapped creatures exchange states simultaneously, while a noncreature is
/// left alone.
#[test]
fn breaking_wave_toggles_both_players_creatures_once() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![index::GRIZZLY_BEARS, index::HILL_GIANT];
    board.extend([island(); 6]);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &board)
        .battlefield(1, &[index::GRAY_OGRE, index::CRAW_WURM, plains()])
        .hand(0, &[breaking_wave()])
        .start();
    keep_mulligans(&mut engine);
    let my_up = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let my_down = on_battlefield(&engine, p0, index::HILL_GIANT).unwrap();
    let their_up = on_battlefield(&engine, p1, index::GRAY_OGRE).unwrap();
    let their_down = on_battlefield(&engine, p1, index::CRAW_WURM).unwrap();
    let noncreature = on_battlefield(&engine, p1, plains()).unwrap();

    assert!(walk_to_own_main(&mut engine, p1), "p1 reaches its own main");
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("p1 gives p0 priority");
    for id in [my_down, their_down, noncreature] {
        engine
            .dev_state_mut(p0)
            .expect("the harness may set tapped states")
            .set_tapped(id, true);
    }
    tap_all_mana(&mut engine, p0);
    cast_sorcery_with_optional_flash(&mut engine, p0, breaking_wave());
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, my_up));
    assert!(!is_tapped(&engine, my_down));
    assert!(is_tapped(&engine, their_up));
    assert!(!is_tapped(&engine, their_down));
    assert!(is_tapped(&engine, noncreature));
}
