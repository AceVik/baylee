//! `cards/sorceries/mv_1/cateran_summons.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cateran Summons — {B} sorcery: "Search your library for a Mercenary card,
/// reveal that card, put it into your hand, then shuffle."
///
/// The library is built hostile on purpose: it is the kit's filler deck, so
/// every card in it is a Forest and not one of them is a Mercenary. That is
/// exactly what the search's own option list is asked about — a list that is
/// non-empty and yet offers nothing is the printed subtype filter doing the
/// excluding, where a search that had lost `Filter::HasSubtype` would offer
/// all of the fillers instead. With no legal find the spell resolves the way a
/// failable search does: no card changes zone, and the Summons itself is the
/// only card that moved, to its owner's graveyard, off a {B} the pool really
/// paid rather than one a board reading assumed.
#[test]
fn cateran_summons_offers_nothing_from_a_library_that_holds_no_mercenary() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[cateran_summons()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, cateran_summons()).expect("the Summons is in hand");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Mana before the claim: `castable` is filtered through the pool, so the
    // Swamp is tapped first and what is asserted is the printed {B}.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the lone Swamp taps for the {{B}} the sorcery costs"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.castable.contains(&spell),
        "with {{B}} in the pool the sorcery is castable: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, cateran_summons());

    // The search, and the rest of the spell behind it. Answering with nothing
    // is what a player does when the filter has nothing to offer; the option
    // list itself is the assertion, and fillers listed here would be a search
    // that never read the word "Mercenary" at all.
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    ChoicePrompt::SearchLibrary,
                    "the card's one instruction is a library search"
                );
                assert!(
                    options.is_empty(),
                    "the library holds {library_before} filler cards and not one \
                     of them is a Mercenary: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                    .expect("a search with nothing to find is answered with nothing");
            }
            Pending::Priority { .. } if stack_is_empty(&engine) => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Summons resolves: {other:?}"),
        }
    }

    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "nothing was found: the library is the length it was, shuffled or not"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the Summons left the hand and no card arrived in its place"
    );
    assert!(
        in_graveyard(&engine, p0, cateran_summons()).is_some(),
        "a sorcery that resolved is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{B}} was really spent on it"
    );
}
