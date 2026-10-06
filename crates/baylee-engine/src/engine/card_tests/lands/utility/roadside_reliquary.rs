//! `cards/lands/utility/roadside_reliquary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Roadside Reliquary: "{T}: Add {C}." / "{2}, {T}, Sacrifice this land: Draw a card if you control an artifact. Draw a card if you control an enchantment."
/// Under `Coverage::Partial`, conditional board branching in effects is unsupported and the sacrifice ability is omitted.
/// Activating ability 0 adds one colorless mana to the pool and taps the land.
#[test]
fn roadside_reliquary_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(134, forest())
        .battlefield(0, &[roadside_reliquary()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land =
        on_battlefield(&engine, p0, roadside_reliquary()).expect("Roadside Reliquary deployed");
    activate(&mut engine, p0, roadside_reliquary(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Roadside Reliquary: "{2}, {T}, Sacrifice this land: Draw a card if you
/// control an artifact. Draw a card if you control an enchantment." Two
/// sentences and not one, which is a claim three boards have to settle: a
/// bare board draws nothing, an artifact alone draws one, and both together
/// draw two. The land is in the graveyard before either sentence runs — it
/// paid for the ability — and is neither kind of permanent, so its
/// departure changes no answer.
#[test]
fn roadside_reliquary_reads_its_two_clauses_one_at_a_time() {
    let p0 = PlayerId::new(0);

    let drawn = |extra: &[CardIndex], seed: u64| -> usize {
        let mut board = vec![roadside_reliquary(), forest(), forest()];
        board.extend_from_slice(extra);
        let mut engine = Duel::new(seed, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let land =
            on_battlefield(&engine, p0, roadside_reliquary()).expect("the Reliquary is seated");
        tap_mana_except(&mut engine, p0, land);
        let before = library_size(&engine, p0);
        activate(&mut engine, p0, roadside_reliquary(), 1);
        assert!(
            on_battlefield(&engine, p0, roadside_reliquary()).is_none(),
            "the sacrifice is a cost and is paid on announcement"
        );
        pass_until(&mut engine, stack_is_empty);
        before - library_size(&engine, p0)
    };

    assert_eq!(drawn(&[], 9213), 0, "neither clause is earned");
    assert_eq!(
        drawn(&[basilisk_collar()], 9214),
        1,
        "the artifact clause alone"
    );
    assert_eq!(
        drawn(&[luminarch_ascension()], 9215),
        1,
        "the enchantment clause alone"
    );
    assert_eq!(
        drawn(&[basilisk_collar(), luminarch_ascension()], 9216),
        2,
        "both clauses, which is what makes them two sentences"
    );
}
