//! `cards/sorceries/mv_1/serum_visions.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Serum Visions — {U} sorcery: "Draw a card. Scry 2."
///
/// The two printed sentences run in one resolution and each leaves its mark
/// somewhere different, which is why they are read off the same cast: the draw
/// is the object that was on top of the library now sitting in hand, and the
/// scry is the question that follows it. The question has to be asked about
/// the top two cards *after* the draw — `second` and `third` of the library as
/// it stood before the cast — so a scry that looked at the pre-draw top, or a
/// draw that happened second, would name the wrong two. The library is one card
/// shorter afterwards and the card the search offered lies on the bottom, which
/// is what tells a reorder with a look from a draw.
#[test]
fn serum_visions_draws_a_card_and_then_scries_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[serum_visions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The library as it stands before anything is cast: the list's last entry
    // is the top card, its first is the bottom — the order `Effect::Scry`
    // reads the top `n` in and the end `ZonePosition::Bottom` writes to.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];
    let third = library_before[library_before.len() - 3];

    cast_from_hand(&mut engine, p0, serum_visions());

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
    assert_eq!(player, p0, "the caster does the looking");
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(
        cards,
        vec![second, third],
        "the top two cards *after* the draw: the card that was on top is in \
         hand and must not be on the scry's menu"
    );
    assert_eq!(
        piles,
        scry_piles(2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(p0, look_answer(&cards, &[second]))
        .expect("one of the two the scry just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(second),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(third),
        "the card left alone is the new top"
    );
    assert_eq!(
        library.len(),
        library_before.len() - 1,
        "\"Draw a card\" took one off the top and a scry only reorders"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "the drawn card is the very object that was on top, which the filler \
         deck's identical printings cannot stand in for"
    );
    assert!(
        in_graveyard(&engine, p0, serum_visions()).is_some(),
        "and the sorcery itself resolved into its owner's graveyard"
    );
}
