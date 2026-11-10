//! `cards/creatures/artifacts/mv_2/wretched_doll.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn wretched_doll() -> CardIndex {
    card_index("628cf9ab-8f8b-49b6-b761-2450088ad03b")
}

/// Wretched Doll: a 3/1 with "{B}, {T}: Surveil 1." Green mana alone does not
/// pay the `{B}`; black does, and the Doll taps.
#[test]
fn wretched_doll_surveils_one_for_black_and_a_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[wretched_doll(), forest(), swamp()], &[]);
    let doll = on_battlefield(&engine, p0, wretched_doll()).expect("seated");
    assert_eq!(pt(&engine, doll), (3, 1), "the printed body");
    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(
        !priority_offer(&engine).abilities.contains(&(doll, 0)),
        "green is not {{B}}"
    );

    tap_mana_where(&mut engine, p0, |id| id != doll);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    activate(&mut engine, p0, wretched_doll(), 0);
    assert!(is_tapped(&engine, doll));

    let library = library_size(&engine, p0);
    let cards = surveil_asked(&mut engine, 1);
    engine
        .apply(p0, look_answer(&cards, &[cards[0]]))
        .expect("to the graveyard");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library - 1);
    assert_eq!(graveyard_count(&engine, 0), 1);
}
