//! `cards/creatures/artifacts/mv_5/tek.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tek() -> CardIndex {
    card_index("f85aef9b-b6ad-4716-b48b-5e26b474a59d")
}

/// Tek — each basic land type you control switches one thing on: Plains
/// +0/+2, Island flying, Swamp +2/+0, Mountain first strike, Forest trample.
/// Alone it is a 2/2 without keywords; each land is checked on its own.
#[test]
fn tek_reads_each_land_type_it_is_printed_with() {
    let p0 = PlayerId::new(0);
    let cases = [
        (plains(), (2, 4), KeywordSet::EMPTY),
        (island(), (2, 2), KeywordSet::FLYING),
        (swamp(), (4, 2), KeywordSet::EMPTY),
        (mountain(), (2, 2), KeywordSet::FIRST_STRIKE),
        (forest(), (2, 2), KeywordSet::TRAMPLE),
    ];
    for (land, body, granted) in cases {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[tek(), land])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let tek_id = on_battlefield(&engine, p0, tek()).expect("seated");
        assert_eq!(pt(&engine, tek_id), body);
        assert_eq!(keywords(&engine, tek_id), granted);
    }
}

/// With all five, everything applies at once; with the same lands under the
/// opponent's control, nothing does ("as long as you control").
#[test]
fn tek_with_every_land_is_a_flying_first_striking_trampling_four_four() {
    let p0 = PlayerId::new(0);
    let lands = [plains(), island(), swamp(), mountain(), forest()];
    let mut mine = vec![tek()];
    mine.extend(lands);
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &mine).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tek_id = on_battlefield(&engine, p0, tek()).expect("seated");
    assert_eq!(pt(&engine, tek_id), (4, 4));
    assert_eq!(
        keywords(&engine, tek_id),
        KeywordSet::FLYING
            .union(KeywordSet::FIRST_STRIKE)
            .union(KeywordSet::TRAMPLE)
    );

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tek()])
        .battlefield(1, &lands)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tek_id = on_battlefield(&engine, p0, tek()).expect("seated");
    assert_eq!(pt(&engine, tek_id), (2, 2));
    assert_eq!(keywords(&engine, tek_id), KeywordSet::EMPTY);
}
