//! `cards/lands/fetch/mountain_valley.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mountain Valley prints two lines — "This land enters tapped" and
/// "{T}, Sacrifice this land: Search your library for a Mountain or Forest
/// card, put it onto the battlefield, then shuffle" — and the first is only
/// readable through what it stops: the land is played as a land drop rather
/// than placed, so it lies tapped and its own `{T}` is unpayable until its
/// controller's next untap step, which is why the offer is empty on arrival
/// and full a turn later. The search is played against the kit's own Forest
/// filler deck, so every card in the library is a legal find and the option
/// list is the whole library. What that find then proves is the
/// destination: the card arrives on the battlefield rather than in hand, the
/// land that paid for it is in the graveyard, and the library is one shorter.
#[test]
fn mountain_valley_enters_tapped_and_trades_itself_for_a_forest_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[mountain_valley()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let valley = play_land(&mut engine, p0, mountain_valley());
    assert!(
        entered_tapped(&engine, valley),
        "\"This land enters tapped\""
    );
    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. }
                if legal.abilities.iter().any(|(source, _)| *source == valley)
        )
    };
    assert!(
        !offered(&engine),
        "and the {{T}} in its own cost is unpayable while it lies tapped, so \
         the land offers nothing at all on the turn it arrives"
    );

    // CR 502.3: nothing stands it back up before its controller's own untap
    // step, which is the next turn it gets. The walk answers every question
    // on the way — including a cleanup discard, which a turn boundary can
    // raise for either seat.
    let played_on = engine.state().turn.number;
    for _ in 0..400 {
        if engine.state().turn.number > played_on
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == p0
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            break;
        }
        let (player, action) = answer_one(&engine).expect("the walk answers what it is asked");
        engine
            .apply(player, action)
            .expect("the answer came out of the question");
    }
    assert!(
        engine.state().turn.number > played_on,
        "the game reaches p0's next turn"
    );
    assert!(
        !is_tapped(&engine, valley),
        "the untap step stands the land back up"
    );
    assert!(
        offered(&engine),
        "so its one line is on the table now: {:?}",
        engine.pending()
    );

    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, mountain_valley(), 0);

    // The search is what the ability *does*, not what it costs: activating it
    // only puts it on the stack, so the question arrives one resolution later.
    // `reach_the_search` is the same walk every other fetchland in this module
    // takes, and it panics rather than returning quietly if the ability never
    // resolves.
    let (options, min, max) =
        reach_the_search(&mut engine).expect("the Valley's one ability searches");
    assert_eq!(
        (min, max),
        (1, 1),
        "one card, and the search is not optional"
    );
    assert_eq!(
        options.len(),
        library_before,
        "every card in the kit's filler deck is a Forest, and the filter \
         admits every Forest, so the whole library is on the menu"
    );

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("the card the search offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "\"put it onto the battlefield\" — the find arrives in play and not \
         in hand"
    );
    assert!(
        on_battlefield(&engine, p0, mountain_valley()).is_none(),
        "the sacrifice took the land itself and nothing beside it"
    );
    assert!(
        in_graveyard(&engine, p0, mountain_valley()).is_some(),
        "and it is in its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the searched card left the library, so shuffling it back is not \
         what happened"
    );
}
