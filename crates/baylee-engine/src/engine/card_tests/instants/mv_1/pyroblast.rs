//! `cards/instants/mv_1/pyroblast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn pyroblast() -> CardIndex {
    card_index("ecc435e2-deb1-420a-a79f-01dd08747314")
}

/// Pyroblast (ecc435e2-deb1-420a-a79f-01dd08747314) — {R} Instant:
/// - "Choose one —"
/// - "• Counter target spell if it's blue."
/// - "• Destroy target permanent if it's blue."
#[test]
fn pyroblast_destroys_blue_permanent_and_leaves_nonblue_permanent_alone() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::ZEPHYR_FALCON, index::GRIZZLY_BEARS],
        &[pyroblast(), pyroblast()],
        &[],
    );
    let blue_falcon = lk::obj(&engine, P1, index::ZEPHYR_FALCON);
    let green_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);
    lk::cast_modal(&mut engine, P0, pyroblast(), 1, 0, &[blue_falcon], &[]);
    assert_eq!(lk::zone_of(&engine, blue_falcon), Zone::Graveyard);

    lk::cast_modal(&mut engine, P0, pyroblast(), 1, 0, &[green_bears], &[]);
    assert_eq!(
        lk::zone_of(&engine, green_bears),
        Zone::Battlefield,
        "Pyroblast does not destroy a non-blue permanent"
    );
}

#[test]
fn pyroblast_counters_a_blue_spell_and_leaves_a_nonblue_spell_alone() {
    // 1. Counters a blue spell (Unsummon targeting Grizzly Bears).
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[pyroblast()],
        &[index::UNSUMMON],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    lk::pass_once(&mut engine);
    lk::float(&mut engine, P1, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P1, index::UNSUMMON);
    lk::aim(&mut engine, &[bears], &[]);
    let unsummon_on_stack = on_stack(&engine, index::UNSUMMON).unwrap();
    lk::pass_once(&mut engine);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::announce(
        &mut engine,
        P0,
        pyroblast(),
        0,
        0,
        &[unsummon_on_stack],
        &[],
    );
    lk::settle(&mut engine);
    assert_eq!(
        lk::zone_of(&engine, bears),
        Zone::Battlefield,
        "blue spell was countered by Pyroblast mode 0, so Grizzly Bears stayed on the battlefield"
    );

    // 2. Targets a non-blue spell (Giant Growth) and leaves it uncountered.
    let mut engine2 = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[pyroblast()],
        &[index::GIANT_GROWTH],
    );
    let bears2 = lk::obj(&engine2, P1, index::GRIZZLY_BEARS);
    lk::pass_once(&mut engine2);
    lk::float(&mut engine2, P1, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut engine2, P1, index::GIANT_GROWTH);
    lk::aim(&mut engine2, &[bears2], &[]);
    let growth_on_stack = on_stack(&engine2, index::GIANT_GROWTH).unwrap();
    lk::pass_once(&mut engine2);
    lk::float(&mut engine2, P0, &[(ManaColor::Red, 1)]);
    lk::announce(&mut engine2, P0, pyroblast(), 0, 0, &[growth_on_stack], &[]);
    lk::settle(&mut engine2);
    assert_eq!(
        power_of(&engine2, bears2),
        Some(5),
        "non-blue spell is not countered by Pyroblast mode 0"
    );
}
