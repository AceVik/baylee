//! `cards/artifacts/mv_3/chromatic_lantern.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Simulacrum Synthesizer ({2}{U}): "When this artifact enters, scry 2.
/// Whenever **another** artifact you control with mana value 3 or greater
/// enters, create a 0/0 colorless Construct artifact creature token with
/// 'This token gets +1/+1 for each artifact you control.'"
///
/// Both printed sentences are played in one first main phase, off one
/// tapping of six Islands: a mana pool empties when a step or phase ends
/// (CR 500.5) and this test never leaves that phase, so the {3} left over
/// from casting the Synthesizer is what the second artifact is cast with.
///
/// The word the second sentence turns on is `another`, and it is struck
/// first: the Synthesizer is itself an artifact of mana value 3 entering
/// under its own controller, so a filter without that word would hand out
/// a Construct beside its own scry. The board is read after the entry
/// trigger has finished, and there is no token on it.
///
/// Then Chromatic Lantern, which is the {3} artifact the card is written
/// about, and the Construct that follows it is a **3/3** — the Synthesizer,
/// the Lantern, and the token itself, which is an artifact creature and so
/// counts itself. Two readings hold that number down from either side. The
/// opponent's two artifacts do not count, because the modifier counts what
/// the effect's controller controls and the card prints "each artifact
/// **you** control": three, never five. And exactly one token arrives — the
/// Construct is another artifact you control entering, but a token has no
/// mana cost, and the mana value of an object with no mana cost is 0
/// (CR 202.3a), so it is never an artifact "with mana value 3 or greater"
/// and cannot feed the ability that made it.
///
/// The scry half is asserted as a **move** and not as a question that was
/// asked: the card chosen off the top lies on the bottom afterwards, the one
/// left alone is the new top card, and the library is the length it was —
/// scry looks and reorders, and draws nothing.
#[test]
fn simulacrum_synthesizer_scries_on_arrival_and_builds_only_for_another_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[simulacrum_synthesizer(), chromatic_lantern()])
        .battlefield(1, &[quiet_artifact(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The two cards the scry is about to look at, named before anything is
    // cast. The list's last entry is the top of the library and its first is
    // the bottom — the order `Effect::Scry` reads the top `n` in, and the
    // end `ZonePosition::Bottom` writes to.
    let library_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    cast_from_hand(&mut engine, p0, simulacrum_synthesizer());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Synthesizer's controller does the looking");
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(cards, vec![top, second], "the top two cards, top first");
    assert_eq!(
        piles,
        scry_piles(2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("one of the two just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the other is the new top"
    );
    assert_eq!(library.len(), library_before.len(), "scry draws nothing");

    // `another`: a mana value 3 artifact just entered under p0's control and
    // it was the Synthesizer itself, so the second ability must not see it.
    let synthesizer = on_battlefield(&engine, p0, simulacrum_synthesizer());
    assert!(synthesizer.is_some(), "the Synthesizer resolved");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and built nothing for itself"
    );

    // {3} of the six Islands is still floating, and the Lantern is the other
    // artifact — mana value 3 exactly — that the second sentence is about.
    cast_from_hand(&mut engine, p0, chromatic_lantern());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        1,
        "one Construct for the Lantern, and none for the Construct itself"
    );
    let construct = tokens[0];
    let kinds = types(&engine, construct);
    assert!(
        kinds.contains(baylee_core::types::TypeSet::ARTIFACT)
            && kinds.contains(baylee_core::types::TypeSet::CREATURE),
        "the token counts itself because it is an artifact creature: {kinds:?}"
    );
    assert_eq!(
        (artifacts_of(&engine, p0), artifacts_of(&engine, p1)),
        (3, 2),
        "Synthesizer, Lantern and Construct on this side; two on the other"
    );
    assert_eq!(
        pt(&engine, construct),
        (3, 3),
        "+1/+1 for each artifact *you* control: three, and never the five \
         standing on the battlefield"
    );
}
