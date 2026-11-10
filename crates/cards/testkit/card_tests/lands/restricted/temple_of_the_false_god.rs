//! `cards/lands/restricted/temple_of_the_false_god.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Temple of the False God and Cryptic Caves: "Activate only if you control
/// five or more lands."
///
/// The other shape `Condition::ControlCount` takes — a threshold rather
/// than "at least one" — and the fifth land is played to cross it, so an
/// off-by-one in either direction is visible. The Temple is the pool's one
/// land that prints *nothing* but a conditional ability, which is why it is
/// worth its own row: there is no unconditional half to fall back on, and a
/// clause read as always-false would leave it a land that does nothing at
/// all.
#[test]
fn the_fifth_land_is_what_turns_a_count_of_five_true() {
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(940, forest())
        .battlefield(
            0,
            &[temple_of_the_false_god(), forest(), forest(), forest()],
        )
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        !offered(&engine, temple_of_the_false_god(), 0),
        "four lands is not five"
    );
    play_land(&mut engine, p0, forest());
    assert!(
        offered(&engine, temple_of_the_false_god(), 0),
        "and the fifth is"
    );
    activate(&mut engine, p0, temple_of_the_false_god(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "{{C}}{{C}}, both of them"
    );

    // Cryptic Caves counts the same way and then spends itself.
    let mut engine = Duel::new(941, forest())
        .battlefield(0, &[cryptic_caves(), forest(), forest(), forest()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // The mana comes first, and that ordering is the test: an ability whose
    // `{1}` nobody can pay is missing from the offer for a reason that has
    // nothing to do with its clause, and asserting on the difference would
    // then prove only that the Forests were untapped. The Caves is kept back
    // because its `{1}, {T}, Sacrifice` is the offer under test and
    // `tap_all_mana` would spend the `{T}` half on mana (#159).
    tap_all_mana_but(&mut engine, p0, Some(cryptic_caves()));
    assert!(
        !offered(&engine, cryptic_caves(), 1),
        "four lands is not five"
    );
    play_land(&mut engine, p0, forest());
    assert!(offered(&engine, cryptic_caves(), 1), "and the fifth is");

    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, cryptic_caves(), 1);
    for _ in 0..4 {
        if engine.state().zones.list(ZoneLocation::Hand(p0)).len() > hand {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected while resolving: {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1,
        "the card it drew"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "and the land it sacrificed to draw it"
    );
}
