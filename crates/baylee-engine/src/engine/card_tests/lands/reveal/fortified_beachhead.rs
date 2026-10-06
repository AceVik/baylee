//! `cards/lands/reveal/fortified_beachhead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fortified Beachhead prints `As this land enters, you may reveal a Soldier card
/// from your hand. This land enters tapped unless you revealed a Soldier card this
/// way or you control a Soldier.`, `{{T}}: Add {{W}} or {{U}}.`, and `{{5}}, {{T}}: Soldiers
/// you control get +1/+1 until end of turn.`
///
/// Under `Coverage::Partial`, revealing a card from hand as an enter modifier is
/// omitted, so the land enters tapped unless its controller controls a Soldier.
/// This test seats an `earth_king_s_lieutenant()`, plays Fortified Beachhead to
/// verify that it enters untapped, and activates ability 0 to choose white mana.
#[test]
fn fortified_beachhead_enters_untapped_with_soldier_and_adds_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[earth_king_s_lieutenant()])
        .hand(0, &[fortified_beachhead()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, fortified_beachhead());
    assert!(
        !entered_tapped(&engine, land),
        "enters untapped with a Soldier"
    );

    activate(&mut engine, p0, fortified_beachhead(), 0);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![ManaColor::White, ManaColor::Blue]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
