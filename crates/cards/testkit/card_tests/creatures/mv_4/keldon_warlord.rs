//! `cards/creatures/mv_4/keldon_warlord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Keldon Warlord — "Keldon Warlord's power and toughness are each equal
/// to the number of non-Wall creatures you control." A Wall and an
/// opponent's creature both fail to count.
#[test]
fn keldon_warlords_power_and_toughness_count_its_controllers_non_wall_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[keldon_warlord(), pearled_unicorn(), wall_of_fire()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let warlord = on_battlefield(&engine, p0, keldon_warlord()).expect("seated");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "seated"
    );
    assert_eq!(
        pt(&engine, warlord),
        (2, 2),
        "itself and the Unicorn; the Wall and the opponent's Elf do not count"
    );
}
