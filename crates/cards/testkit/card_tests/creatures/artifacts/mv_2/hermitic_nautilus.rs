//! `cards/creatures/artifacts/mv_2/hermitic_nautilus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hermitic_nautilus() -> CardIndex {
    card_index("6bda307b-1508-4582-952b-c690db85e045")
}

/// Hermitic Nautilus: a 1/4 with "Vigilance". It attacks without tapping.
#[test]
fn hermitic_nautilus_attacks_without_tapping() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[hermitic_nautilus()], &[]);
    let nautilus = on_battlefield(&engine, p0, hermitic_nautilus()).expect("seated");
    assert_eq!(pt(&engine, nautilus), (1, 4));
    assert!(keywords(&engine, nautilus).contains(KeywordSet::VIGILANCE));
    swing_unblocked(&mut engine, nautilus);
    assert_eq!(life_of(&engine, 1), 19);
    assert!(!is_tapped(&engine, nautilus));
}

/// "{1}{U}: This creature gets +3/-3 until end of turn": one activation makes
/// it a 4/1; a second makes toughness -2 and it dies; the first one's effect
/// ends with the turn.
#[test]
fn hermitic_nautilus_gets_plus_three_minus_three_per_activation() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[],
        &[hermitic_nautilus(), island(), island(), island(), island()],
        &[],
    );
    let nautilus = on_battlefield(&engine, p0, hermitic_nautilus()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, hermitic_nautilus(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, nautilus), (4, 1), "+3/-3");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{1}}{{U}} paid"
    );

    activate(&mut engine, p0, hermitic_nautilus(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, hermitic_nautilus()).is_none(),
        "toughness -2: it died"
    );
    assert!(in_graveyard(&engine, p0, hermitic_nautilus()).is_some());
}

/// The pump is "until end of turn": a single activation is gone next turn.
#[test]
fn hermitic_nautilus_pump_ends_with_the_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[hermitic_nautilus(), island(), forest()], &[]);
    let nautilus = on_battlefield(&engine, p0, hermitic_nautilus()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, hermitic_nautilus(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, nautilus), (4, 1));
    to_my_next_turn(&mut engine);
    assert_eq!(pt(&engine, nautilus), (1, 4));
}
