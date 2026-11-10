//! `cards/creatures/mv_7/elesh_norn_grand_cenobite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn elesh_norn_grand_cenobite() -> CardIndex {
    card_index("958d71ff-c9f7-46f0-96ca-79e7f4d65a16")
}

/// Elesh Norn, Grand Cenobite — a 4/7 vigilance for {5}{W}{W}. "Creatures your
/// opponents control get -2/-2": cast, she kills the opposing 2/2 on the
/// spot by state-based action.
#[test]
fn elesh_norn_kills_their_two_two_when_she_arrives() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(
        &lots(plains(), 7),
        &[elesh_norn_grand_cenobite()],
        &[gray_ogre()],
    );
    let elesh = cast_and_resolve(&mut engine, elesh_norn_grand_cenobite());
    assert_body(&engine, elesh, (4, 7), KeywordSet::VIGILANCE);
    assert!(
        in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "a 2/2 with -2/-2 has toughness 0"
    );
    let _ = p0;
}

/// Both anthems on one board: her own other creatures get +2/+2, hers
/// does not, and a bigger opposing creature is only shrunk.
#[test]
fn elesh_norn_pumps_mine_and_shrinks_theirs() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let engine = table7(
        &[elesh_norn_grand_cenobite(), gray_ogre()],
        &[],
        &[earth_elemental()],
    );
    let elesh = on_battlefield(&engine, p0, elesh_norn_grand_cenobite()).expect("out");
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    let theirs = on_battlefield(&engine, p1, earth_elemental()).expect("out");
    assert_eq!(pt(&engine, elesh), (4, 7), "\"other\" creatures only");
    assert_eq!(pt(&engine, mine), (4, 4), "+2/+2");
    assert_eq!(pt(&engine, theirs), (2, 3), "-2/-2 on a 4/5");
}

/// Vigilance: she attacks and stays untapped.
#[test]
fn elesh_norn_attacks_without_tapping() {
    let mut engine = table7(&[elesh_norn_grand_cenobite()], &[], &[]);
    let elesh =
        on_battlefield(&engine, PlayerId::new(0), elesh_norn_grand_cenobite()).expect("out");
    fight(&mut engine, &[elesh], &[]);
    assert_eq!(life_of(&engine, 1), 16);
    assert!(!is_tapped(&engine, elesh));
}
