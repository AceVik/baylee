//! `cards/creatures/mv_3/opposition_agent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1f438b8f-fe23-4f3b-ab2e-f6c33676c462"

/// Opposition Agent — {2}{B} — Creature — Human Rogue, printed 3/2 with flash,
/// and one static: "You control your opponents while they're searching their
/// libraries. While an opponent is searching their library, they exile each
/// card they find. You may play those cards for as long as they remain exiled …"
///
/// The scenario plays the static on the opponent's own turn: p0's Agent is
/// already on the battlefield, p1 casts a tutor, and the takeover makes p0 —
/// not the searcher — answer the search. What proves it is the destination:
/// the found card is in exile and the searcher's hand did not grow, which is
/// the half of "search your library for a card, put that card into your hand"
/// the takeover exists to break.
#[test]
fn opposition_agent_takes_over_the_search_and_exiles_what_the_opponent_finds() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // The Agent is seated from the start — this is about its static, not its
    // cast — and the opponent's own three Swamps pay for their tutor.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[opposition_agent()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[grim_tutor()])
        .start();
    keep_mulligans(&mut engine);

    let agent = on_battlefield(&engine, p0, opposition_agent()).expect("the Agent is seated");
    assert_eq!(pt(&engine, agent), (3, 2), "the printed 3/2 body");
    assert!(
        keywords(&engine, agent).contains(KeywordSet::FLASH),
        "the printed flash reaches the permanent"
    );

    // p1 takes their own main phase and casts the tutor off the three Swamps;
    // the search question follows on the same turn the Agent was already there.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, grim_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on nothing else")
    };
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert_eq!(
        player, p0,
        "\"you control your opponents while they're searching\": the Agent's \
         controller answers the search, not the searcher"
    );
    assert!(!options.is_empty(), "the library it searches holds cards");
    let library = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    for id in &options {
        assert!(
            library.contains(id),
            "the search still runs over p1's own library: {id:?} is no card in it"
        );
    }

    let chosen = options[0];
    let searcher_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p1))
            .contains(&chosen),
        "\"they exile each card they find\": the found card never reaches the \
         searcher's hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        searcher_hand,
        "the hand did not grow, which a plain tutor would have made it do"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&chosen)
            || engine
                .state()
                .zones
                .list(ZoneLocation::Exile(p1))
                .contains(&chosen),
        "and the card went to exile, which is where the Agent's controller may \
         play it from"
    );
}
