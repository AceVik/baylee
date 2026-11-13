//! `cards/lands/utility/command_tower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

/// Katara's identity supplies exactly white, blue and green, including while
/// the commander is still in the command zone. The no-commander case is played
/// by `command_tower_without_a_commander_makes_no_mana` in the engine.
#[test]
fn command_tower_offers_only_its_commanders_colors() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1_203, forest())
        .commander(0, &[katara_the_fearless()])
        .battlefield(0, &[index::COMMAND_TOWER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, index::COMMAND_TOWER, 0);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("a colored commander must offer its identity's colors");
    };
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue, ManaColor::Green]
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(
        &engine,
        on_battlefield(&engine, p0, index::COMMAND_TOWER).unwrap()
    ));
}
