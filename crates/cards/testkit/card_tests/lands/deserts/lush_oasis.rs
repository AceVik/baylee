//! `cards/lands/deserts/lush_oasis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lush Oasis — a Desert land: "This land enters tapped", "When this land
/// enters, it deals 1 damage to target opponent", "{T}: Add {G} or {U}".
///
/// One game reads all three printed lines, and each needs the game to say so
/// rather than the card file: the land is *played*, so the entry replacement
/// (CR 614.1c) and the enters-trigger are the engine's own answers; the
/// trigger can only be aimed across the table, which is why the life that
/// moves is the opponent's and never its own controller's; and a turn cycle
/// is spent before the mana line is read, because a land that arrived tapped
/// offers no `{T}` at all — the untap step is what makes the offer evidence
/// instead of a tautology.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn lush_oasis_enters_tapped_burns_an_opponent_and_taps_for_green_or_blue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lush_oasis()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    let card = in_hand(&engine, p0, lush_oasis()).expect("the Oasis is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land drop on an empty board");
    let oasis = on_battlefield(&engine, p0, lush_oasis()).expect("the Oasis landed");
    assert!(
        is_tapped(&engine, oasis),
        "\"This land enters tapped\" — the entry modifier, not a placement"
    );

    // The enters-trigger is put on the stack with its target chosen
    // (CR 603.3d), so the question arrives before anyone may respond.
    let mut aimed = false;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the land's controller does the aiming");
                assert!(
                    player_options.contains(&p1) && !player_options.contains(&p0),
                    "\"target opponent\" is the seat across the table, and \
                     never the one that played the land: {player_options:?}"
                );
                assert_eq!((min, max), (1, 1), "one opponent, asked for once");
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: Vec::new(),
                            players: vec![p1],
                        },
                    )
                    .expect("the only option the question offered");
                aimed = true;
                break;
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, p0, "the land's controller does the aiming");
                assert!(
                    options.contains(&p1) && !options.contains(&p0),
                    "\"target opponent\" is the seat across the table, and \
                     never the one that played the land: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .expect("the only option the question offered");
                aimed = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the enters-trigger resolves: {other:?}"),
        }
    }
    assert!(aimed, "the enters-trigger asks for its target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"it deals 1 damage to target opponent\""
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the land's own controller is not an opponent it could point at"
    );

    // A whole turn cycle: the land came in tapped, so its `{T}` is not even
    // offered until the untap step has handed it back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let oasis = on_battlefield(&engine, p0, lush_oasis()).expect("still on the table");
    assert!(!is_tapped(&engine, oasis), "the untap step gave it back");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is p0's own main phase");
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == oasis)
        .expect("an untapped land with a printed mana ability is offered its {T}");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("{{T}} and nothing else");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{U}}\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that taps is the one that names it");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "both colours the card prints: {options:?}"
    );
    assert_eq!(options.len(), 2, "and no third: {options:?}");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of them");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, oasis), "the land paid its own {{T}}");
}
