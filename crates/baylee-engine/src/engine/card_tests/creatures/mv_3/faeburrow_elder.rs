//! `cards/creatures/mv_3/faeburrow_elder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Faeburrow Elder prints vigilance, "+1/+1 for each color among permanents
/// you control" and a `{T}` ability adding one mana of each of those colors —
/// and its head is `Coverage::Partial`, with both colour-count lines left off
/// the card. That leaves the vigilance, and the only place it can be read is
/// the projection of a permanent the harness put down: the printed body is
/// 0/0, and the line that would grow it is the missing one, so whoever
/// receives priority first lets CR 704.5f bury it. Both halves are one
/// scenario — the layer system projects the vigilance onto the 0/0, and the
/// same card cast for real resolves, dies, and lies in its owner's graveyard
/// without ever having been able to attack.
#[test]
fn faeburrow_elder_projects_vigilance_onto_a_zero_zero_the_missing_line_never_grows() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[faeburrow_elder(), forest(), forest(), plains()])
        .hand(0, &[faeburrow_elder()])
        .start();

    // Read before the first priority round, which is the one window a 0/0
    // permanent exists in.
    let seeded = on_battlefield(&engine, p0, faeburrow_elder()).expect("the harness put it down");
    let printed = keywords(&engine, seeded);
    assert!(
        printed.contains(KeywordSet::VIGILANCE),
        "the printed vigilance is projected onto the permanent: {printed:?}"
    );
    assert_eq!(
        pt(&engine, seeded),
        (0, 0),
        "and the body is the printed 0/0: nothing counts the colours among \
         permanents you control, because that line is the `Coverage::Partial` gap"
    );

    // The first priority round is where CR 704.5f reads that body.
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );
    assert!(
        on_battlefield(&engine, p0, faeburrow_elder()).is_none(),
        "a 0/0 with no counters is put into its owner's graveyard before \
         anybody may use the vigilance it has"
    );

    // And the same card cast for real, which is what the gap costs the player
    // who draws it: it resolves and is buried again.
    cast_from_hand(&mut engine, p0, faeburrow_elder());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, faeburrow_elder()).is_none(),
        "the freshly cast Elder is not on the battlefield either"
    );
    assert!(
        in_graveyard(&engine, p0, faeburrow_elder()).is_some(),
        "it resolved and went to its owner's graveyard, as a 0/0 does"
    );
}
