//! `cards/lands/deserts/cactus_preserve.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::color::{Color, ColorSet};

/// Cactus Preserve: "This land enters tapped." / "{T}: Add one mana of any type that a land you control could produce." / "{3}: Until end of turn, this land becomes an X/X green Plant creature..."
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it produces red mana matching a controlled Mountain.
#[test]
fn cactus_preserve_enters_tapped_and_taps_for_controlled_land_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(306, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[cactus_preserve()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, cactus_preserve());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, cactus_preserve(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn cactus_preserve_animates_using_the_greatest_commander_mana_value() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .commander(0, &[index::GABRIEL_ANGELFIRE, index::SAVANNAH_LIONS])
        .battlefield(0, &[cactus_preserve(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let largest = e.state().commanders[0][0].object;
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            largest,
            ZoneLocation::Hand(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    let land = on_battlefield(&e, p0, cactus_preserve()).unwrap();
    tap_mana_except(&mut e, p0, land);
    activate(&mut e, p0, cactus_preserve(), 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, land), (7, 7));
    assert!(keywords(&e, land).contains(KeywordSet::REACH));
    let c = e.state().object(land).unwrap().characteristics();
    assert!(c.types.contains(TypeSet::CREATURE));
    assert!(c.types.contains(TypeSet::LAND));
    assert_eq!(c.colors, ColorSet::of(Color::Green));
    reach_their_main_phase(&mut e, p1);
    assert!(
        !e.state()
            .object(land)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}

/// "This land becomes an X/X green Plant creature … It's still a land": the
/// animation adds the Plant subtype, and the Desert it already was stays a
/// Desert (it only gains, nothing is replaced).
#[test]
fn cactus_preserve_animated_is_a_plant_and_still_a_desert() {
    use baylee_core::generated::{index, subtypes};
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .commander(0, &[index::GABRIEL_ANGELFIRE])
        .battlefield(0, &[cactus_preserve(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, cactus_preserve()).unwrap();
    let before = e.state().object(land).unwrap().characteristics().subtypes;
    assert!(
        before.contains(subtypes::land::DESERT) && !before.contains(subtypes::creature::PLANT),
        "control: before the animation it is a Desert and no Plant"
    );
    tap_mana_except(&mut e, p0, land);
    activate(&mut e, p0, cactus_preserve(), 1);
    pass_until(&mut e, stack_is_empty);
    let c = e.state().object(land).unwrap().characteristics();
    assert!(c.types.contains(TypeSet::CREATURE), "it is animated");
    assert!(
        c.subtypes.contains(subtypes::creature::PLANT),
        "a Plant creature"
    );
    assert!(
        c.subtypes.contains(subtypes::land::DESERT),
        "and still a Desert: \"It's still a land\""
    );
}
