//! `cards/artifacts/mv_0/mox_opal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mox Opal: "Metalcraft — {T}: Add one mana of any color. Activate only if
/// you control three or more artifacts." The Mox and two Sol Rings are
/// three; the Mox and one are two.
#[test]
fn mox_opal_taps_for_mana_only_with_three_artifacts() {
    let p0 = PlayerId::new(0);
    let offered = |ring_count: usize| -> (Engine<RegistryLookup>, ObjectId, bool) {
        let mut board = vec![mox_opal()];
        board.extend(std::iter::repeat_n(sol_ring(), ring_count));
        let mut engine = Duel::new(4702, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let mox = on_battlefield(&engine, p0, mox_opal()).expect("the Mox");
        let yes = priority_offer(&engine).abilities.contains(&(mox, 0));
        (engine, mox, yes)
    };

    let (_, _, with_two) = offered(1);
    assert!(!with_two, "two artifacts are not metalcraft");

    let (mut engine, mox, with_three) = offered(2);
    assert!(with_three, "three artifacts are");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mox,
                ability_index: 0,
            },
        )
        .expect("the Mox taps");
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("any color asks, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "all five colors: {options:?}");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
}
