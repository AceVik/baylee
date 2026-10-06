//! `cards/sorceries/mv_3/grim_tutor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Grim Tutor — {1}{B}{B} sorcery: "Search your library for a card, put that
/// card into your hand, then shuffle. You lose 3 life."
///
/// Two effects with a library between them, so the only reading worth playing
/// is both halves off one cast: three Swamps pay the cost, the search question
/// is answered with the card that was on top, and the three life are gone
/// whatever the search found. Naming the top card before anything is cast is
/// what makes the search a *move* rather than a question that was asked — the
/// library is one card shorter and that exact object is in hand, where a tutor
/// that asked and then did nothing would leave both where they were. The
/// opponent's life is the control for "you".
#[test]
fn grim_tutor_finds_the_card_it_names_and_charges_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(19, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[grim_tutor()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The card the search will be answered with, named before anything is
    // cast: the *last* entry of the list is the top of the library.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library to search");
    let top_card = engine
        .state()
        .object(top)
        .and_then(|o| o.card)
        .expect("the top of a library is a card")
        .index;
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, grim_tutor());

    // Let the spell resolve; the first thing it asks is which card.
    let mut asked_for_the_card = false;
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the caster searches their own library");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::SearchLibrary,
                    "a tutor is a search and not a discard or a scry"
                );
                assert!(min >= 1 && max >= 1, "a card is found, not looked at");
                assert!(
                    options.contains(&top),
                    "`Filter::Any` puts the whole library on the menu: {options:?}"
                );
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects: vec![top] })
                    .expect("the card the question offered is one of its answers");
                asked_for_the_card = true;
                break;
            }
            other => panic!("unexpected while the Tutor resolves: {other:?}"),
        }
    }
    assert!(
        asked_for_the_card,
        "Grim Tutor asks which card before it moves one anywhere"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        17,
        "\"you lose 3 life\", off the same resolution as the search"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the caster: the opponent pays nothing"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "the found card left the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the exact object that was offered, now in hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Tutor left the hand and the card it found took its place"
    );
    assert!(
        in_hand(&engine, p0, top_card).is_some(),
        "read by printing too, so the object identity is not the only evidence"
    );
}
