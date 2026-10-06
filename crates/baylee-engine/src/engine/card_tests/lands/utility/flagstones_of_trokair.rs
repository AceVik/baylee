//! `cards/lands/utility/flagstones_of_trokair.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flagstones of Trokair: a legendary land that taps for {W} and, when it is
/// put into a graveyard from the battlefield, may search its controller's
/// library for a Plains card and put it onto the battlefield tapped.
///
/// The second sentence is played by playing a **second** Flagstones rather
/// than by destroying the first: two legendary permanents sharing a name are
/// what CR 704.5j puts into a graveyard, so the zone change the trigger is
/// written about comes out of the rules and not out of the harness, and no
/// card beyond the one under test has to be named to reach it. The library is
/// Plains, so the search has something to find, and both halves are then read
/// off the board — the fetched land arrived *tapped*, and the Flagstones still
/// standing taps for the one white mana nothing else on the table could make.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn flagstones_of_trokair_bins_its_legend_twin_and_fetches_a_tapped_plains() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[flagstones_of_trokair()])
        .hand(0, &[flagstones_of_trokair()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let first =
        on_battlefield(&engine, p0, flagstones_of_trokair()).expect("one Flagstones stands");
    let library_before = library_size(&engine, p0);

    // The second copy is a land drop and costs nothing; the moment it is on
    // the table the legend rule has work to do.
    let played = play_land(&mut engine, p0, flagstones_of_trokair());

    let mut legend_options: Vec<ObjectId> = Vec::new();
    let mut search_options: Vec<ObjectId> = Vec::new();
    let mut fetched: Option<ObjectId> = None;
    for _ in 0..80 {
        match engine.pending().clone() {
            Pending::LegendChoice { player, options } => {
                assert_eq!(player, p0, "the controller of the two copies is asked");
                legend_options = options.clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![played],
                        },
                    )
                    .expect("the copy that was named was one of the two");
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } => {
                engine
                    .apply(player, PlayerAction::YesNo(true))
                    .expect("\"you may search\" is answered yes");
            }
            Pending::ChooseCards {
                player,
                options,
                prompt: crate::choice::ChoicePrompt::SearchLibrary,
                ..
            } => {
                let choice = *options.first().expect("a Plains deck has a Plains to find");
                search_options = options.clone();
                fetched = Some(choice);
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![choice],
                        },
                    )
                    .expect("the card the search offered is the card it accepts");
            }
            Pending::Priority { player, .. } if stack_is_empty(&engine) && player == p0 => break,
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("priority passes");
            }
            other => panic!("unexpected while the Flagstones' trigger resolves: {other:?}"),
        }
    }

    assert_eq!(
        legend_options.len(),
        2,
        "both copies were on the battlefield at once: {legend_options:?}"
    );
    assert!(
        legend_options.contains(&first) && legend_options.contains(&played),
        "and the choice is between exactly those two: {legend_options:?}"
    );
    assert!(
        in_graveyard(&engine, p0, flagstones_of_trokair()).is_some(),
        "CR 704.5j put one of them into its owner's graveyard — the zone \
         change the second sentence is written about"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, flagstones_of_trokair()).len(),
        1,
        "and left the other one standing"
    );

    assert!(
        search_options.len() > 1,
        "the death trigger offered a library search over the library itself, \
         not a single card: {search_options:?}"
    );
    let fetched = fetched.expect("a card was chosen out of the search");
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the found card left the library"
    );
    let landed = on_battlefield(&engine, p0, plains()).expect("the searched Plains entered");
    assert_eq!(
        landed, fetched,
        "the card the search offered is the card that landed"
    );
    assert!(
        entered_tapped(&engine, landed),
        "\"put it onto the battlefield tapped\""
    );

    // The mana half, read after the search has been paid for. The Plains the
    // search just delivered is kept back, so the white in the pool can only
    // have come off the Flagstones still standing.
    let taken = tap_mana_except(&mut engine, p0, landed);
    assert_eq!(
        taken, 1,
        "one mana route here: the Flagstones, and not the Plains that just arrived"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "{{T}}: Add {{W}}, and no other permanent on this board makes white"
    );
    assert!(
        is_tapped(&engine, landed),
        "\"put it onto the battlefield tapped\" — a searched Plains arrives \
         the way the trigger says, not the way a land drop would"
    );
}
