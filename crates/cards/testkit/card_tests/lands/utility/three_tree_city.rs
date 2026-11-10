//! `cards/lands/utility/three_tree_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Three Tree City` prints `As Three Tree City enters, choose a creature type.`, `{{T}}: Add {{C}}.`, and `{{2}}, {{T}}: Choose a color. Add an amount of mana of that color equal to the number of creatures you control of the chosen type.`
///
/// Marked `Coverage::Implemented`, `Three Tree City` asks for a creature subtype via `Pending::ChooseSubtype` upon entering the battlefield.
/// With two controlled `young_wolf()` creatures and `baylee_core::generated::subtypes::creature::WOLF` chosen, activating ability 1 with `{{2}}` floating mana prompts via `Pending::ChooseColor` and adds two mana of the chosen color.
#[test]
fn three_tree_city_chooses_subtype_and_produces_mana_scaled_by_creature_count() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[three_tree_city()])
        .battlefield(0, &[young_wolf(), young_wolf(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, three_tree_city()).expect("three tree city in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!("expected ChooseSubtype, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    let wolf = baylee_core::generated::subtypes::creature::WOLF;
    assert!(options.contains(&wolf));
    engine.apply(p0, PlayerAction::ChooseSubtype(wolf)).unwrap();

    let city = on_battlefield(&engine, p0, three_tree_city()).expect("city on battlefield");
    assert!(!entered_tapped(&engine, city));

    // Float {{2}} green mana from two basic forests while keeping Three Tree City untapped.
    tap_mana_except(&mut engine, p0, city);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2
    );
    assert!(!is_tapped(&engine, city));

    activate(&mut engine, p0, three_tree_city(), 1);
    let Pending::ChooseColor {
        options: color_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(color_opts.len(), 5, "all five mana colors are selectable");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        2,
        "two red mana added for two controlled Wolves"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the two green mana were spent paying {{2}}"
    );
    assert_eq!(pool.total(), 2);
    assert!(is_tapped(&engine, city));
}
