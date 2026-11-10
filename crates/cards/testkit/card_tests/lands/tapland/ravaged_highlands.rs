//! `cards/lands/tapland/ravaged_highlands.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ravaged Highlands` enters tapped, taps for `{{R}}`, and sacrifices for `{{T}}` to add one mana
/// of any color under `Coverage::Implemented`.
/// After entering tapped and untapping on the following turn, activating ability 1 without floating
/// any mana prompts for a color choice via `Pending::ChooseColor`.
/// Choosing black mana sacrifices the land to the graveyard and adds exactly one black mana to
/// an otherwise empty mana pool.
#[test]
fn ravaged_highlands_enters_tapped_and_sacrifices_for_any_color() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(3306, forest())
        .hand(0, &[ravaged_highlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, ravaged_highlands());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, ravaged_highlands(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert!(in_graveyard(&engine, p0, ravaged_highlands()).is_some());
    assert!(on_battlefield(&engine, p0, ravaged_highlands()).is_none());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
}
