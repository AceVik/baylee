//! `cards/creatures/artifacts/mv_2/one_clown_band.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn one_clown_band() -> CardIndex {
    card_index("eeaa03c9-533f-4067-a2c8-86f3b9e58583")
}

/// Buzz Bots, a Robot Insect, the other Robot on the table.
fn buzz_bots() -> CardIndex {
    card_index("434e720f-2bfa-49b6-a5ac-fe0c0b24764d")
}

/// One-Clown Band: a 1/3 with "{2}{R}: Target Robot gets +2/+0 until end of
/// turn". The menu holds the Robots (the Band itself and Buzz Bots) and not
/// the Bears; aimed at Buzz Bots it becomes a 3/1.
#[test]
fn one_clown_band_gives_a_target_robot_plus_two_power() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[],
        &[
            one_clown_band(),
            buzz_bots(),
            grizzly_bears(),
            mountain(),
            forest(),
            forest(),
        ],
        &[],
    );
    let band = on_battlefield(&engine, p0, one_clown_band()).expect("seated");
    let bots = on_battlefield(&engine, p0, buzz_bots()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, band), (1, 3));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, one_clown_band(), 0);
    let options = aim_at(&mut engine, p0, bots);
    assert!(options.contains(&band) && options.contains(&bots), "Robots");
    assert!(!options.contains(&bears), "the Bears are no Robot");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bots), (3, 1), "+2/+0");
    to_my_next_turn(&mut engine);
    assert_eq!(pt(&engine, bots), (1, 1), "until end of turn");
}

/// A Band that targets itself is a 3/3.
#[test]
fn one_clown_band_may_target_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[],
        &[one_clown_band(), mountain(), forest(), forest()],
        &[],
    );
    let band = on_battlefield(&engine, p0, one_clown_band()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, one_clown_band(), 0);
    aim_at(&mut engine, p0, band);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, band), (3, 3));
}

/// Red is required: three Forests cannot pay `{2}{R}`.
#[test]
fn one_clown_band_needs_red() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[one_clown_band(), forest(), forest(), forest()], &[]);
    let band = on_battlefield(&engine, p0, one_clown_band()).expect("seated");
    tap_all_mana(&mut engine, p0);
    assert!(!priority_offer(&engine).abilities.contains(&(band, 0)));
}
