//! `cards/lands/restricted/dark_fortress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dark Fortress: "{T}: Add {C}." / "{T}: Add {B} or {R}. Activate only if this land entered this turn or if you control a basic land."
/// Ability 0 taps the land for {C}; the conditional ability is
/// `the_gathering_place_sentence_holds_on_dark_fortress_and_training_compound`.
#[test]
fn dark_fortress_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .battlefield(0, &[dark_fortress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fortress = on_battlefield(&engine, p0, dark_fortress()).expect("Dark Fortress deployed");
    activate(&mut engine, p0, dark_fortress(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, fortress));
}

/// Dark Fortress and Training Compound print Gathering Place's sentence in
/// other colours: "{T}: Add {B} or {R}" / "{R} or {G}. Activate only if this
/// land entered this turn or if you control a basic land."
///
/// Each half of the "or" is read alone. On the turn the land is played
/// there is no basic, so the entry clause is what offers the ability; a
/// turn later it has not entered this turn and there is still no basic, so
/// it is withheld; a Forest then answers the other half.
#[test]
fn the_gathering_place_sentence_holds_on_dark_fortress_and_training_compound() {
    let p0 = PlayerId::new(0);
    let offered = |engine: &Engine<RegistryLookup>, land: ObjectId, index: u32| {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending());
        };
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == land && *idx == index)
    };
    for (card, colors) in [
        (dark_fortress(), [ManaColor::Black, ManaColor::Red]),
        (training_compound(), [ManaColor::Red, ManaColor::Green]),
    ] {
        let mut engine = Duel::new(SEED, forest()).hand(0, &[card, forest()]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, card);
        assert!(
            offered(&engine, land, 1),
            "the turn it entered, with no basic land: the entry clause"
        );
        activate(&mut engine, p0, card, 1);
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("expected a colour choice, got {:?}", engine.pending());
        };
        assert_eq!(options.len(), 2, "exactly the two printed colours");
        assert!(colors.iter().all(|c| options.contains(c)), "{options:?}");
        engine
            .apply(p0, PlayerAction::ChooseColor(colors[1]))
            .expect("a colour the question offered");
        assert_eq!(engine.state().players[0].mana_pool.available(colors[1]), 1);

        reach_their_main_phase(&mut engine, PlayerId::new(1));
        reach_their_main_phase(&mut engine, p0);
        assert!(
            !offered(&engine, land, 1),
            "a turn later, still with no basic land: withheld"
        );
        assert!(offered(&engine, land, 0), "{{T}}: Add {{C}} stays");

        let _ = play_land(&mut engine, p0, forest());
        assert!(
            offered(&engine, land, 1),
            "a basic land answers the other half of the \"or\""
        );
    }
}
