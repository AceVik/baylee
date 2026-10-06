//! `cards/lands/triome/spara_s_headquarters.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spara's Headquarters — `Land — Forest Plains Island`: "({T}: Add {G}, {W},
/// or {U}.)", "This land enters tapped", "Cycling {3}".
///
/// All three printed lines are played in one game because none of them is
/// legible from the card file. A land the harness *places* is not a land that
/// enters — `starting_battlefield` is a placement, so no replacement looks at
/// it and it would arrive untapped whatever the card says — so the triome is
/// played for real, read as tapped, and the empty offer on it is read in the
/// same breath, because a land that came in tapped has no `{T}` left to pay
/// with. The copy left in hand is then cycled for three Forests and pays its
/// own price: the card into its owner's graveyard, one card off the library.
/// A turn later the untap step has stood the battlefield copy back up, and its
/// `{T}` buys exactly one mana out of the three colours it prints — with the
/// three Forests still standing, so the mana on the pool cannot be theirs.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, three lines, and a turn between two of them
fn sparas_headquarters_enters_tapped_cycles_itself_and_taps_for_one_of_its_three_colors() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[spara_s_headquarters(), spara_s_headquarters()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Every mana source on the board, named before anything is tapped: they are
    // the control for "the land made that mana" further down.
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 3, "three Forests were dealt");

    // Cycling {3} is an ability of a card *in hand* (ActivationZone::Hand), and
    // `can_afford` reads the pool rather than the untapped Forests, so the mana
    // is floating before anything is claimed about the offer.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests tapped for the {{3}} the cycling charges"
    );

    // Ability 0 is the mana line and ability 1 is the cycling.
    activate(&mut engine, p0, spara_s_headquarters(), 1);
    assert!(
        in_graveyard(&engine, p0, spara_s_headquarters()).is_some(),
        "\"Discard this card\" is part of the cost, so it is paid as the ability \
         is activated (CR 601.2h)"
    );
    assert!(
        !stack_is_empty(&engine),
        "cycling is no mana ability, so the draw waits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn, so the hand is the size it was — and \
         the card that paid is in the graveyard rather than here"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} came out of the pool"
    );

    // The second copy, played the way a land is played.
    let land = play_land(&mut engine, p0, spara_s_headquarters());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a main phase, got {:?}", engine.pending())
    };
    assert!(
        deeds(&legal, &[land]).is_empty(),
        "a land that came in tapped has no {{T}} left to pay with: {legal:?}"
    );

    // And it gives nothing in the turn it arrived, so the mana line is read a
    // turn later: only the untap step turns its {{T}} into an ability at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // Both lists an offer can carry a mana route in are read: a land with a
    // basic land type is named by the CR 305.6 shortcut, which carries no index
    // to point at, while a land printing its own `{T}: Add ...` is an ordinary
    // `(source, index)` entry. The printed line is taken when it is there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let routes = deeds(&legal, &[land]);
    assert!(
        !routes.is_empty(),
        "the triome is a mana source this offer names, one way or the other: {legal:?}"
    );
    let route = routes
        .iter()
        .map(|(_, deed)| *deed)
        .find(|deed| matches!(deed, Deed::Ability(_)))
        .unwrap_or(routes[0].1);
    engine
        .apply(p0, route.action(land))
        .expect("a route the offer named is one the seat may take");

    // The colour is a question and never a default, and the menu is exactly the
    // three the card prints — asked before the mana is anywhere.
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseColor { player, options } => {
                assert_eq!(player, p0, "the seat that tapped names the colour");
                assert_eq!(
                    options.len(),
                    3,
                    "\"Add {{G}}, {{W}}, or {{U}}\" is three colours and no fourth: {options:?}"
                );
                for color in [ManaColor::Green, ManaColor::White, ManaColor::Blue] {
                    assert!(
                        options.contains(&color),
                        "one of the three the card prints is missing from the menu: {options:?}"
                    );
                }
                assert!(
                    !options.contains(&ManaColor::Black) && !options.contains(&ManaColor::Red),
                    "and neither of the two it does not print: {options:?}"
                );
                engine
                    .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
                    .expect("blue is one of the three it offered");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected while the land is being tapped: {other:?}"),
        }
    }

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        pool.available(ManaColor::Green)
            + pool.available(ManaColor::White)
            + pool.available(ManaColor::Blue),
        1,
        "one mana of one of the three colours the land prints"
    );
    assert_eq!(
        pool.available(ManaColor::Black) + pool.available(ManaColor::Red),
        0,
        "and never a colour it does not"
    );
    assert!(
        forests.iter().all(|id| !is_tapped(&engine, *id)),
        "the Forests are still standing, so the mana on the pool was the land's \
         and not a basic's"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
}
