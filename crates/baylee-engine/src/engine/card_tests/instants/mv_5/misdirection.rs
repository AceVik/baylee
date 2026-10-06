//! `cards/instants/mv_5/misdirection.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Misdirection is {3}{U}{U} for two printed sentences — "You may exile a blue
/// card from your hand rather than pay this spell's mana cost" and "Change the
/// target of target spell with a single target" — and neither is readable off
/// the card file. p1 therefore owns **no land at all**: the pitched Counterspell
/// is the whole of the price, so the empty pool is how clause one is read. The
/// Swords to Plowshares p0 aimed across the table is then turned onto p0's own
/// Elf, which is clause two: the exile says the target really moved, and the one
/// life says the spell is still p0's, because a redirected spell pays its own
/// controller and not the seat that re-aimed it.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn misdirection_pitches_a_blue_card_to_turn_the_swords_onto_my_own_elves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), llanowar_elves()])
        .hand(0, &[swords_to_plowshares()])
        // No land under p1: the printed cost is unpayable here, so the
        // alternative cost is the only way this spell can arrive at all.
        .battlefield(1, &[llanowar_elves()])
        .hand(1, &[misdirection(), counterspell()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // (1) The spell that is about to be redirected. p0's Swords names the Elf
    // across the table, which is where its life payment would have shown.
    cast_from_hand(&mut engine, p0, swords_to_plowshares());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Swords to Plowshares targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    let swords_spell =
        on_stack(&engine, swords_to_plowshares()).expect("the Swords is waiting on the stack");

    // (2) The response, and the card under test. `pass_until` hands priority
    // to the seat that may answer the spell that was just cast.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "p1 owns no land at all, so the pitch cost is the whole of what this \
         spell can cost"
    );
    let pitch = in_hand(&engine, p1, counterspell()).expect("the blue card to pitch is in hand");
    cast_with_floating(&mut engine, p1, misdirection());

    // Clause one's cost, clause two's own target and clause two's new target
    // arrive in whatever order the engine asks them; each is answered where it
    // is, and the two target questions are told apart by what they enumerate.
    let mut turned = false;
    for _ in 0..24 {
        match engine.pending().clone() {
            // The redirect's own question, asked of the seat that cast
            // Misdirection. A menu holding a creature is that question: the
            // spell's own target choice below enumerates a spell instead.
            Pending::ChooseTargets {
                player, options, ..
            } if options.contains(&mine) || options.contains(&theirs) => {
                assert_eq!(
                    player, p1,
                    "the controller of the redirect names the new target, not \
                     the controller of the spell"
                );
                assert!(
                    options.contains(&mine) && !options.contains(&theirs),
                    "the new target is **another** creature (CR 115.7a): the one \
                     it names now is not offered again: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![mine],
                        },
                    )
                    .expect("the new target came out of the menu it published");
                turned = true;
                break;
            }
            // {3}{U}{U} against a pitch cost, and only one of them payable on
            // a board with no mana.
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                    .expect("the pitch cost is one of the ways to cast this card");
                engine
                    .apply(player, PlayerAction::ChooseMode(slot))
                    .expect("the pitch cost is a legal way to pay it");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p1, "the seat paying the cost answers it");
                assert_eq!((min, max), (1, 1), "one blue card, no more and no fewer");
                assert!(
                    options.contains(&pitch),
                    "the blue card in hand is the whole of the price: {options:?}"
                );
                for id in &options {
                    assert!(
                        engine.state().object(*id).is_some_and(|o| {
                            o.characteristics()
                                .colors
                                .contains(baylee_core::color::Color::Blue)
                        }),
                        "\"exile a **blue** card from your hand\": {id:?} is not one"
                    );
                }
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![pitch],
                        },
                    )
                    .expect("the card the cost offered pays it");
            }
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p1, "the seat casting Misdirection aims it");
                assert!(
                    options.contains(&swords_spell),
                    "\"target spell\" is the Swords that is on the stack: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![swords_spell],
                        },
                    )
                    .expect("the Swords was one of the spells it offered");
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing priority is always legal");
            }
            other => panic!("unexpected while Misdirection is being cast: {other:?}"),
        }
    }
    assert!(
        turned,
        "the redirect asked for the spell's new target, which is the whole card"
    );
    pass_until(&mut engine, stack_is_empty);

    // (3) Clause two, read where it lands rather than where it was asked: the
    // spell now names p0's Elf, so that is the creature that goes to exile.
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the creature the Swords was turned onto left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))),
        "and it is in *exile*, which is where Swords to Plowshares sends a creature"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell was originally aimed at never moved, so the \
         target really changed rather than the spell merely resolving"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"its controller gains life equal to its power\": the Swords is still \
         p0's, so p0 gains the one the Elf was worth"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the seat that re-aimed the spell pays nothing for it"
    );

    // (4) Clause one, read the same way: a blue card out of the hand into
    // exile, and no mana produced anywhere.
    assert!(
        in_hand(&engine, p1, counterspell()).is_none(),
        "the pitched card left the hand"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == counterspell()))),
        "and it is exiled, which is the whole of the alternative cost"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "no mana was ever produced, because p1 owns no land at all"
    );
    assert!(
        in_graveyard(&engine, p1, misdirection()).is_some(),
        "the instant resolved into its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, swords_to_plowshares()).is_some(),
        "and so did the spell it re-aimed"
    );
}

/// Misdirection on a Lightning Bolt (#247). "Any target" is a player too
/// (CR 115.4), and the redirect used to offer no player at all. Three boards,
/// one for each way a target can move between a player and an object:
/// - aimed at p1, turned onto p0, the other player;
/// - aimed at p1, turned onto p0's Elves;
/// - aimed at p0's Elves, turned onto p1.
#[test]
fn misdirection_turns_a_bolt_between_faces_and_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (from_elves, to_elves, lives, elves_live) in [
        (false, false, (17, 20), true),
        (false, true, (20, 20), false),
        (true, false, (20, 17), true),
    ] {
        let board = format!("from the Elves: {from_elves}, to the Elves: {to_elves}");
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[mountain(), llanowar_elves()])
            .hand(0, &[lightning_bolt()])
            .hand(1, &[misdirection(), counterspell()])
            .life(0, 20)
            .life(1, 20)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");

        let bolt = if from_elves {
            aimed(&mut engine, p0, lightning_bolt(), &[elves], &[])
        } else {
            aimed(&mut engine, p0, lightning_bolt(), &[], &[p1])
        };
        let Some(Pending::ChooseTargets {
            options,
            player_options,
            ..
        }) = misdirected(&mut engine, p1, bolt)
        else {
            panic!("{board}: the Bolt has other legal targets, so the new one is asked for")
        };
        if from_elves {
            assert!(
                !options.contains(&elves) && player_options == vec![p0, p1],
                "{board}: either player, and not the Elves it names: \
                 {options:?} {player_options:?}"
            );
        } else {
            assert!(
                options.contains(&elves) && player_options == vec![p0],
                "{board}: the Elves, and the player it is not aimed at: \
                 {options:?} {player_options:?}"
            );
        }
        let (objects, players) = if to_elves {
            (vec![elves], vec![])
        } else if from_elves {
            (vec![], vec![p1])
        } else {
            (vec![], vec![p0])
        };
        engine
            .apply(p1, PlayerAction::ChooseTargets { objects, players })
            .expect("a target the question offered");
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            (
                engine.state().players[0].life,
                engine.state().players[1].life
            ),
            lives,
            "{board}: the life totals"
        );
        assert_eq!(
            on_battlefield(&engine, p0, llanowar_elves()).is_some(),
            elves_live,
            "{board}: whether the Elves took the 3"
        );
    }
}

/// Misdirection on a Counterspell (#247). The Counterspell's legal targets
/// are spells, which the redirect never walked: it looked at the battlefield
/// only. Two spells are left out: the Swords the Counterspell names (not
/// "another" target) and the Counterspell itself (CR 115.5). What is left is
/// the Misdirection, still on the stack as it resolves, and turning the
/// Counterspell onto it lets the Swords through.
#[test]
fn misdirection_turns_a_counterspell_onto_itself_and_the_swords_resolves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[swords_to_plowshares(), misdirection(), counterspell()])
        .battlefield(1, &[island(), island(), llanowar_elves()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    let swords = aimed(&mut engine, p0, swords_to_plowshares(), &[theirs], &[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    let counter = aimed(&mut engine, p1, counterspell(), &[swords], &[]);
    let Some(Pending::ChooseTargets {
        options,
        player_options,
        ..
    }) = misdirected(&mut engine, p0, counter)
    else {
        panic!("the Misdirection is a spell on the stack, so there is another target")
    };
    let misdirection = on_stack(&engine, misdirection()).expect("it is resolving");
    assert_eq!(
        (options, player_options),
        (vec![misdirection], vec![]),
        "neither the Swords it names nor the Counterspell itself"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![misdirection],
            },
        )
        .expect("the one spell offered");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the Counterspell lost its target when the Misdirection left the \
         stack, and the Swords exiled the Elves"
    );
}

/// Misdirection on a Swords to Plowshares whose Elves are the only creature
/// on the table: there is no other legal target, so the target stays (CR
/// 115.7a), and nothing is asked. It stays **even if it is illegal by then**,
/// which is the second board: the Elves are gone before Misdirection
/// resolves, and the Swords, still aimed at them, fizzles (CR 608.2b).
#[test]
fn misdirection_leaves_the_swords_on_the_only_creature_even_once_it_is_gone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for gone in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[plains()])
            .hand(0, &[swords_to_plowshares()])
            .battlefield(1, &[llanowar_elves()])
            .hand(1, &[misdirection(), counterspell()])
            .life(0, 20)
            .life(1, 20)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

        let swords = aimed(&mut engine, p0, swords_to_plowshares(), &[theirs], &[]);
        if gone {
            let state = engine
                .dev_state_mut(p0)
                .expect("the harness may set boards up");
            state
                .move_object(
                    theirs,
                    ZoneLocation::Exile(p1),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .expect("the harness moves the Elves");
            engine.refresh_offer();
        }
        let asked = misdirected(&mut engine, p1, swords);
        assert!(
            asked.is_none(),
            "gone = {gone}: no other creature, so no new target is asked for: {asked:?}"
        );
        assert_eq!(
            engine
                .state()
                .object(swords)
                .expect("the Swords is still on the stack")
                .targets
                .to_vec(),
            vec![theirs],
            "gone = {gone}: the Swords still names the Elves"
        );
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().players[1].life,
            if gone { 20 } else { 21 },
            "gone = {gone}: the Swords exiles the Elves it names and their \
             controller gains 1, or, with the Elves gone, it fizzles"
        );
    }
}
