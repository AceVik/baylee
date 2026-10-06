//! `cards/creatures/mv_2/scryb_ranger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scryb Ranger — {1}{G} 1/1 Faerie Ranger with flash, flying and "Return a
/// Forest you control to its owner's hand: Untap target creature. Activate
/// only once each turn."
///
/// Every word of the activation is read off an offer rather than the file:
/// the cost menu holds this seat's three Forests and neither the Plains
/// beside them nor the Forest across the table, the target question reaches
/// both sides of the table, and the Forest really leaves the battlefield for
/// the hand while the tapped Elves stands back up. The two Forests still on
/// the table are what make the second activation's absence the printed limit
/// rather than an empty board.
#[allow(clippy::too_many_lines)] // a cost that returns a land, and the once-a-turn limit after it
#[test]
fn scryb_ranger_trades_a_forest_for_one_untap_and_then_its_limit_bites() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(101, island())
        .battlefield(
            0,
            &[forest(), forest(), forest(), plains(), llanowar_elves()],
        )
        .hand(0, &[scryb_ranger()])
        // A Forest and a creature on the other side of the table: both "a
        // Forest you control" and "target creature" have something to
        // decline over there.
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elf is kept back: tapping it is the deliberate move below, and an
    // untap aimed at a creature something else already tapped would prove
    // nothing about this card.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, scryb_ranger());
    pass_until(&mut engine, stack_is_empty);
    let ranger = on_battlefield(&engine, p0, scryb_ranger()).expect("the Ranger resolved");
    assert!(
        keywords(&engine, ranger).contains(KeywordSet::FLYING),
        "flying is a keyword bit on the printed card"
    );

    let land = on_battlefield(&engine, p0, plains()).expect("my Plains is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        3,
        "three Forests to trade"
    );

    // Tap the Elves with its own {T}: an untap aimed at an untapped creature
    // changes nothing and would prove nothing.
    activate(&mut engine, p0, llanowar_elves(), 0);
    assert!(is_tapped(&engine, elves), "the Elves paid its own tap");

    // The protection static is index 0 in the card's list, so the one
    // printed activation is looked up rather than assumed.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(source, _)| *source == ranger)
        .expect("the Ranger's one printed activation is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the activation starts");

    // The two questions one activation asks, answered in the order they
    // arrive rather than the order they are expected.
    let mut asked_target = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if asked_target && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                assert!(
                    player_options.is_empty(),
                    "\"target creature\" is objects only: {player_options:?}"
                );
                assert!(
                    options.contains(&elves) && options.contains(&their_elves),
                    "\"target creature\" reaches both sides of the table: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![elves],
                            players: vec![],
                        },
                    )
                    .unwrap();
                asked_target = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostReturn,
                    "a cost and not a search, which is all a client has to tell \
                     the two apart"
                );
                assert_eq!((min, max), (1, 1), "one Forest, no more and no fewer");
                menu = options.clone();
                let mine = options
                    .iter()
                    .copied()
                    .find(|id| {
                        engine
                            .state()
                            .object(*id)
                            .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
                    })
                    .expect("a Forest of mine is on the menu");
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![mine],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Ranger's activation resolves: {other:?}"),
        }
    }
    assert!(asked_target, "\"untap target creature\" is a target choice");
    assert_eq!(
        menu.len(),
        3,
        "the three Forests this seat controls: {menu:?}"
    );
    assert!(!menu.contains(&land), "a Plains is no Forest: {menu:?}");
    assert!(
        !menu.contains(&their_forest),
        "a seat returns only what it controls: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        2,
        "the Forest left the battlefield for its owner's hand"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "and it is in hand — the library is Islands, so no Forest could have \
         been there before"
    );
    assert!(
        !is_tapped(&engine, elves),
        "\"untap target creature\": the Elves stands back up"
    );
    assert_eq!(
        all_on_battlefield(&engine, p1, forest()).len(),
        1,
        "and the Forest across the table never moved"
    );

    // Once each turn. Two Forests are still on the table — the cost could be
    // paid again — so the absence is the printed limit and not an empty
    // board.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == ranger),
        "\"activate only once each turn\": {:?}",
        legal.abilities
    );
}
