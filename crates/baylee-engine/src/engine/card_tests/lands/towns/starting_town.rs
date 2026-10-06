//! `cards/lands/towns/starting_town.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Starting Town: "This land enters tapped unless it's your first, second, or third turn of the game." / "{T}: Add {C}." / "{T}, Pay 1 life: Add one mana of any color."
/// Under `Coverage::Partial`, counting turns of the game is unsupported and the land enters untapped.
/// Activating ability 1 costs 1 life and produces one mana of any color chosen by the player.
#[test]
fn starting_town_enters_untapped_and_pays_life_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(124, forest()).hand(0, &[starting_town()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, starting_town());
    assert!(!entered_tapped(&engine, land));

    let before_life = engine.state().players[0].life;
    activate(&mut engine, p0, starting_town(), 1);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    assert_eq!(engine.state().players[0].life, before_life - 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}
