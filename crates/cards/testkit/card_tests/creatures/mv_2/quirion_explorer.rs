//! `cards/creatures/mv_2/quirion_explorer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Quirion Explorer — {1}{G} Creature — Elf Druid Scout 1/1
/// "{T}: Add one mana of any color that a land an opponent controls could produce."
#[test]
fn quirion_explorer_adds_mana_of_color_opponent_land_could_produce() {
    let explorer = card_index("766c8a68-9c51-44ea-903c-1f5cb30941d1");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[explorer, forest()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, explorer, 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a color choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Black, ManaColor::Red]);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
}
