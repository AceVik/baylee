//! `cards/creatures/mv_8/avacyn_angel_of_hope.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn avacyn() -> CardIndex {
    card_index("216cb26e-8da9-478b-bfbc-8030f7adee72")
}

/// Avacyn is an 8/8 for {5}{W}{W}{W} with flying, vigilance and
/// indestructible.
#[test]
fn avacyn_is_cast_for_eight_with_flying_vigilance_and_indestructible() {
    let mut engine = at_main(&[plains(); 8], &[avacyn()]);
    let angel = cast_and_settle(&mut engine, avacyn());
    assert_eq!(pt(&engine, angel), (8, 8));
    let k = keywords(&engine, angel);
    assert!(k.contains(KeywordSet::FLYING));
    assert!(k.contains(KeywordSet::VIGILANCE));
    assert!(k.contains(KeywordSet::INDESTRUCTIBLE));
}

/// "Other permanents you control have indestructible": before she arrives
/// the Plains and the Bear have none; after, both do, the opponent's Bear
/// does not.
#[test]
fn avacyn_gives_every_other_permanent_of_hers_indestructible() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[avacyn()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let mine = on_battlefield(&engine, p0, grizzly_bears()).expect("my bear");
    let land = on_battlefield(&engine, p0, plains()).expect("my Plains");
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("their bear");
    for id in [mine, land, theirs] {
        assert!(!keywords(&engine, id).contains(KeywordSet::INDESTRUCTIBLE));
    }
    let angel = cast_and_settle(&mut engine, avacyn());
    assert!(keywords(&engine, mine).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(
        keywords(&engine, land).contains(KeywordSet::INDESTRUCTIBLE),
        "permanents, not only creatures"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::INDESTRUCTIBLE),
        "the opponent's permanent is not hers"
    );
    assert!(keywords(&engine, angel).contains(KeywordSet::INDESTRUCTIBLE));
}
