//! `cards/creatures/mv_7/murktide_regent.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("7f993ac7-c2cd-413c-a106-2c051a77ebf6")
}

fn treasure_cruise() -> CardIndex {
    card_index("5b6bdf5a-2742-4851-92cd-a857a3852836")
}

fn send_to_graveyard(
    engine: &mut Engine<RegistryLookup>,
    owner: PlayerId,
    cards: &[ObjectId],
) -> Vec<ObjectId> {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("dev commands enabled");
    let moved = cards
        .iter()
        .map(|&id| {
            state
                .move_object(
                    id,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::DevCommand,
                )
                .expect("card moves to graveyard")
        })
        .collect();
    engine.refresh_offer();
    moved
}

/// Casting Murktide Regent with delve exiling an instant, a sorcery, and a
/// land pays {3} toward {5}{U}{U}, grants Flying, and enters with +1/+1
/// counters only for the instant and sorcery (not the land). Its own
/// leaves-the-graveyard trigger did not see them leave from the stack.
#[test]
fn murktide_regent_enters_with_counters_only_for_delved_instants_and_sorceries() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 4])
        .hand(0, &[subject(), counterspell(), treasure_cruise(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_cards: Vec<ObjectId> = [counterspell(), treasure_cruise(), forest()]
        .into_iter()
        .map(|card| in_hand(&engine, p0, card).expect("seeded card in hand"))
        .collect();
    let graveyard_cards = send_to_graveyard(&mut engine, p0, &hand_cards);

    tap_mana_where(&mut engine, p0, |_| true);
    let regent_in_hand = in_hand(&engine, p0, subject()).expect("Murktide Regent in hand");
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: regent_in_hand,
            },
        )
        .expect("4 Islands + 3 delve exiles pay {5}{U}{U}");
    let Pending::ChooseCards {
        prompt: ChoicePrompt::Delve,
        ..
    } = engine.pending()
    else {
        panic!("expected delve prompt, got {:?}", engine.pending());
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: graveyard_cards,
            },
        )
        .expect("exiling all three cards is legal");
    pass_until(&mut engine, stack_is_empty);

    let regent = on_battlefield(&engine, p0, subject()).expect("Murktide Regent resolved");
    assert!(keywords(&engine, regent).contains(KeywordSet::FLYING));
    assert_eq!(counters_on(&engine, regent, CounterKind::P1P1), 2);
    assert_eq!(power_of(&engine, regent), Some(5));
}

/// An instant exiled to delve Murktide Regent that leaves exile in response
/// before resolution (even if put back into exile) is no longer exiled with
/// the resolving Dragon.
#[test]
fn murktide_regent_loses_link_if_delved_instant_leaves_exile_before_resolution() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 5])
        .hand(0, &[subject(), counterspell(), treasure_cruise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_cards: Vec<ObjectId> = [counterspell(), treasure_cruise()]
        .into_iter()
        .map(|card| in_hand(&engine, p0, card).expect("seeded card in hand"))
        .collect();
    let graveyard_cards = send_to_graveyard(&mut engine, p0, &hand_cards);

    tap_mana_where(&mut engine, p0, |_| true);
    let regent_in_hand = in_hand(&engine, p0, subject()).expect("Murktide Regent in hand");
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: regent_in_hand,
            },
        )
        .expect("5 Islands + 2 delve exiles pay {5}{U}{U}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: graveyard_cards,
            },
        )
        .expect("exiling both cards is legal");

    let state = engine.dev_state_mut(p0).expect("dev commands");
    let exiled_instant = *state
        .zones
        .list(ZoneLocation::Exile(p0))
        .iter()
        .find(|id| {
            state
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == counterspell())
        })
        .expect("exiled instant");
    let bounced = state
        .move_object(
            exiled_instant,
            ZoneLocation::Hand(p0),
            ZonePosition::Top,
            Cause::DevCommand,
        )
        .unwrap();
    state
        .move_object(
            bounced,
            ZoneLocation::Exile(p0),
            ZonePosition::Top,
            Cause::DevCommand,
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    let regent = on_battlefield(&engine, p0, subject()).expect("Murktide Regent resolved");
    assert_eq!(counters_on(&engine, regent, CounterKind::P1P1), 1);
}

/// On the battlefield, Murktide Regent gets a +1/+1 counter for each instant
/// or sorcery card that leaves your graveyard (e.g. exiled to delve a later
/// spell), ignores non-instant/non-sorcery cards leaving your graveyard, and
/// ignores instants leaving an opponent's graveyard.
#[test]
fn murktide_regent_grows_when_your_instants_and_sorceries_leave_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[subject(), island(), island(), island(), island(), island()],
        )
        .hand(
            0,
            &[
                treasure_cruise(),
                counterspell(),
                treasure_cruise(),
                forest(),
            ],
        )
        .battlefield(1, &[island(); 7])
        .hand(1, &[treasure_cruise(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let regent = on_battlefield(&engine, p0, subject()).expect("Murktide Regent on battlefield");
    assert_eq!(counters_on(&engine, regent, CounterKind::P1P1), 0);

    // Seed p0's graveyard with 1 instant, 1 sorcery, and 1 forest; p1's with 1 instant.
    let p0_instant = in_hand(&engine, p0, counterspell()).expect("p0 instant");
    let p0_sorcery = in_hand(&engine, p0, treasure_cruise()).expect("p0 sorcery");
    let p0_forest = in_hand(&engine, p0, forest()).expect("p0 forest");
    let my_gy = send_to_graveyard(&mut engine, p0, &[p0_instant, p0_sorcery, p0_forest]);
    let p1_instant = in_hand(&engine, p1, counterspell()).expect("p1 instant");
    let opp_gy = send_to_graveyard(&mut engine, p1, &[p1_instant]);

    // Cast Treasure Cruise delving 2 instants/sorceries and 1 forest -> triggers twice!
    tap_mana_where(&mut engine, p0, |_| true);
    let cruise = in_hand(&engine, p0, treasure_cruise()).expect("p0 second Treasure Cruise");
    engine
        .apply(p0, PlayerAction::CastSpell { card: cruise })
        .expect("5 Islands + 3 delves cast Treasure Cruise");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: my_gy })
        .expect("delving all 3 cards is legal");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, regent, CounterKind::P1P1),
        2,
        "only the instant and sorcery leaving your graveyard add counters"
    );
    assert_eq!(power_of(&engine, regent), Some(5));

    // On opponent's turn, opponent delves an instant out of THEIR graveyard -> must NOT trigger p0's Regent.
    reach_their_main_phase(&mut engine, p1);
    tap_mana_where(&mut engine, p1, |_| true);
    let opp_cruise = in_hand(&engine, p1, treasure_cruise()).expect("p1 Treasure Cruise");
    engine
        .apply(p1, PlayerAction::CastSpell { card: opp_cruise })
        .expect("7 Islands + 1 delve cast Treasure Cruise");
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: opp_gy })
        .expect("p1 delves their instant");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, regent, CounterKind::P1P1),
        2,
        "an instant leaving an opponent's graveyard must not trigger your Murktide Regent"
    );
}

/// Casting Murktide Regent a second time without delve enters as a new object
/// with zero counters (CR 400.7).
#[test]
fn murktide_regent_cast_again_without_delve_has_no_linked_exiles() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 7])
        .hand(0, &[subject(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let inst = in_hand(&engine, p0, counterspell()).expect("instant in hand");
    let gy = send_to_graveyard(&mut engine, p0, &[inst]);

    tap_mana_where(&mut engine, p0, |_| true);
    let regent_id = in_hand(&engine, p0, subject()).expect("Murktide Regent in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: regent_id })
        .expect("cast with 7 lands + 1 delve");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: gy })
        .expect("delve 1 instant");
    pass_until(&mut engine, stack_is_empty);
    let regent = on_battlefield(&engine, p0, subject()).expect("on battlefield");
    assert_eq!(counters_on(&engine, regent, CounterKind::P1P1), 1);

    // Bounce back to hand and recast with 7 untapped lands on next turn without delving.
    engine
        .dev_state_mut(p0)
        .expect("dev commands")
        .move_object(
            regent,
            ZoneLocation::Hand(p0),
            ZonePosition::Top,
            Cause::DevCommand,
        )
        .unwrap();
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);

    tap_mana_where(&mut engine, p0, |_| true);
    let regent_id = in_hand(&engine, p0, subject()).expect("back in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: regent_id })
        .expect("recast with 7 lands");
    if let Pending::ChooseCards {
        prompt: ChoicePrompt::Delve,
        ..
    } = engine.pending()
    {
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
            .expect("delve 0");
    }
    pass_until(&mut engine, stack_is_empty);
    let regent = on_battlefield(&engine, p0, subject()).expect("re-entered");
    assert_eq!(counters_on(&engine, regent, CounterKind::P1P1), 0);
}
