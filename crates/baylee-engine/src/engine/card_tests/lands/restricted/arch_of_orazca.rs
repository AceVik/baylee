//! `cards/lands/restricted/arch_of_orazca.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arch of Orazca prints Ascend, `{{T}}: Add {{C}}.`, and `{{5}}, {{T}}: Draw a card.
/// Activate only if you have the city's blessing.`
///
/// Under `Coverage::Partial`, Ascend and the city's blessing activation are omitted
/// because no rule or condition tracks the blessing. This test establishes a board
/// with ten permanents, verifies that only the mana ability is offered rather than
/// an ungated draw ability, and confirms tapping it produces one colorless mana.
#[test]
fn arch_of_orazca_offers_only_mana_ability_with_ten_permanents() {
    let p0 = PlayerId::new(0);
    let mut board = vec![arch_of_orazca()];
    board.extend(std::iter::repeat_n(forest(), 10));

    let mut engine = Duel::new(SEED, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let arch = on_battlefield(&engine, p0, arch_of_orazca()).expect("arch on battlefield");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.abilities.iter().filter(|(id, _)| *id == arch).count(),
        1,
        "only ability 0 is offered under Coverage::Partial"
    );

    activate(&mut engine, p0, arch_of_orazca(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, arch));
}
