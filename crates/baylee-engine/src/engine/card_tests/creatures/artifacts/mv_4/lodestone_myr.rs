//! `cards/creatures/artifacts/mv_4/lodestone_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn lodestone_myr() -> CardIndex {
    card_index("7635fb35-7ac8-4e61-8a52-6b0c14cd88b8")
}

/// Lodestone Myr — {4} 2/2 with trample: "Tap an untapped artifact you
/// control: This creature gets +1/+1 until end of turn." The Myr is itself an
/// artifact, so on a board of the Myr and two Sol Rings it pays three times,
/// 5/5 in the end, each time tapping one more untapped artifact; with every
/// artifact tapped the ability is no longer offered.
#[test]
fn lodestone_myr_taps_an_artifact_per_boost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lodestone_myr(), sol_ring(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let myr = on_battlefield(&engine, p0, lodestone_myr()).expect("seated");
    let rings = all_on_battlefield(&engine, p0, sol_ring());
    assert_eq!(rings.len(), 2);
    assert!(keywords(&engine, myr).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, myr), (2, 2));

    let tapped = |engine: &Engine<RegistryLookup>| {
        [myr, rings[0], rings[1]]
            .into_iter()
            .filter(|id| is_tapped(engine, *id))
            .count()
    };
    for boost in 1..=3 {
        assert_eq!(tapped(&engine), boost - 1, "before boost {boost}");
        activate(&mut engine, p0, lodestone_myr(), 0);
        answer_until_settled(&mut engine);
        assert_eq!(tapped(&engine), boost, "one more artifact tapped");
        let plus = i16::try_from(boost).expect("small");
        assert_eq!(pt(&engine, myr), (2 + plus, 2 + plus));
    }
    assert!(
        !priority_offer(&engine).abilities.contains(&(myr, 0)),
        "no untapped artifact is left to tap"
    );
}
