//! `cards/lands/legendary/minas_tirith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Minas Tirith prints `Minas Tirith enters tapped unless you control a legendary creature`, `{T}: Add {W}`,
/// and `{1}{W}, {T}: Draw a card. Activate only if you attacked with two or more creatures this turn.`
/// The card is marked `Coverage::Partial` because `Condition` cannot count attacking creatures.
/// With controlled legendary creature `thorin_oakenshield` on the battlefield, Minas Tirith enters untapped
/// and taps for `{W}` at ability index 0 while omitting the draw ability.
#[test]
fn minas_tirith_enters_untapped_with_legendary_creature_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[thorin_oakenshield()])
        .hand(0, &[minas_tirith()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, minas_tirith());
    assert!(!entered_tapped(&engine, land));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == land)
        .collect();
    assert_eq!(offered, vec![(land, 0)]);

    activate(&mut engine, p0, minas_tirith(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}
