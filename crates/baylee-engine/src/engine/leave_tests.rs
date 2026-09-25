//! Leaving the game while somebody else is being asked (CR 800.4a, #277).
//!
//! A spell being cast and a resolution waiting on a choice are both moments
//! when nobody has priority (CR 117.2e; CR 601.2i), and a concession takes
//! out of them only what was the leaver's. The question stays with the seat
//! it was asked of, without the leaver or their objects among its options.
//! The resolution goes on from where it stopped, never from its first
//! instruction. Three seats, where a game outlives a player, except where a
//! duel has to end.
//!
//! And after they have gone (#278): a triggered ability they would control
//! is never put on the stack, a delayed one included, and nothing is
//! created for them (CR 800.4d).

use super::testkit::{
    Duel, RegistryLookup, basic_forest, card_index, cast_from_hand, in_graveyard, in_hand,
    keep_mulligans, on_stack, pass_until, quiet_creature, reach_main_phase, stack_is_empty,
    tap_all_mana,
};
use super::*;
use crate::win::Victor;
use baylee_core::ids::CardIndex;

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

/// `{U}` "Draw three cards, then put two cards from your hand on top of your
/// library in any order": a resolution that has done something before it
/// asks.
fn brainstorm() -> CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}

/// `{1}{B}`, first mode "Each opponent sacrifices a nontoken creature of
/// their choice": a resolution that asks one player after another.
fn sheoldreds_edict() -> CardIndex {
    card_index("217062f5-96f1-454c-9507-17f34ef37070")
}

/// Its adventure is Swift Spiral, `{1}{W}` "Exile target nontoken creature.
/// Return it to the battlefield under its owner's control at the beginning
/// of the next end step." Two Plains pay for the adventure and not for the
/// `{2}{U}{U}` creature, so casting the card is casting Swift Spiral.
fn twining_twins() -> CardIndex {
    card_index("105aea98-8eb9-4fb2-a0cb-7c7513317c5b")
}

/// `{U}` "Target player draws three cards."
fn ancestral_recall() -> CardIndex {
    card_index("550c74d4-1fcb-406a-b02a-639a760a4380")
}

fn hand_and_library(engine: &Engine<RegistryLookup>, player: PlayerId) -> (usize, usize) {
    let zones = &engine.state().zones;
    (
        zones.list(ZoneLocation::Hand(player)).len(),
        zones.list(ZoneLocation::Library(player)).len(),
    )
}

fn stack_size(engine: &Engine<RegistryLookup>) -> usize {
    engine.state().zones.list(ZoneLocation::Stack).len()
}

/// The question on the table, spelled out: `Pending` has no `PartialEq`.
fn question(engine: &Engine<RegistryLookup>) -> String {
    format!("{:?}", engine.pending())
}

fn creatures_of(engine: &Engine<RegistryLookup>, player: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == player && o.card.is_some_and(|c| c.index == quiet_creature())
            })
        })
        .collect()
}

/// Answers the question on the table with its first `min` options.
fn choose_the_first(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    let Pending::ChooseCards {
        player: asked,
        options,
        min,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a choice of cards, got {:?}", engine.pending());
    };
    assert_eq!(asked, player);
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: options[..usize::from(min)].to_vec(),
            },
        )
        .unwrap();
}

/// A table of `seats` where seat 1 casts Brainstorm in seat 0's upkeep and
/// it resolves up to its question: seat 1 has drawn its three and is asked
/// which two go back. Returns seat 1's hand and library as the spell was
/// cast.
fn brainstorm_asking_seat_1(seats: usize) -> (Engine<RegistryLookup>, (usize, usize)) {
    let mut engine = Duel::table(277, island(), seats)
        .hand(1, &[brainstorm()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    assert_eq!(engine.pending().asked(), Some(seat(0)));
    engine.apply(seat(0), PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, seat(1), brainstorm());
    let cast = hand_and_library(&engine, seat(1));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    assert_eq!(engine.pending().asked(), Some(seat(1)));
    assert_eq!(
        hand_and_library(&engine, seat(1)),
        (cast.0 + 3, cast.1 - 3),
        "Brainstorm has drawn before it asks"
    );
    (engine, cast)
}

/// Seat 0 casts Sheoldred's Edict's first mode at two opponents with two
/// creatures each, and it resolves up to its first question: seat 1's.
fn edict_asking_seat_1() -> Engine<RegistryLookup> {
    let mut engine = Duel::table(277, swamp(), 3)
        .hand(0, &[sheoldreds_edict()])
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .battlefield(2, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    cast_from_hand(&mut engine, seat(0), sheoldreds_edict());
    engine.apply(seat(0), PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    assert_eq!(engine.pending().asked(), Some(seat(1)));
    engine
}

/// Seat 0, in its main phase with two Plains' worth of mana floating, starts
/// casting Swift Spiral with a creature under each seat in `creatures`, and
/// is asked for its target.
fn swift_spiral_asking_for_a_target(creatures: &[usize]) -> Engine<RegistryLookup> {
    let mut duel = Duel::table(277, plains(), 3)
        .hand(0, &[twining_twins()])
        .battlefield(0, &[plains(), plains()]);
    for &at in creatures {
        duel = duel.battlefield(at, &[quiet_creature()]);
    }
    let mut engine = duel.start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    tap_all_mana(&mut engine, seat(0));
    let card = in_hand(&engine, seat(0), twining_twins()).expect("in hand");
    engine
        .apply(seat(0), PlayerAction::CastSpell { card })
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { player, .. } if *player == seat(0)),
        "{:?}",
        engine.pending()
    );
    engine
}

/// #277's probe. Seat 2 leaves while Brainstorm waits on seat 1. The
/// question stays where it was, nobody is given priority with the spell
/// half-resolved, and Brainstorm's three cards are drawn once: the
/// resolution goes on from its question rather than starting again, which
/// drew six.
#[test]
fn a_bystanders_concession_leaves_a_resolution_where_it_was() {
    let (mut engine, cast) = brainstorm_asking_seat_1(3);
    let asked = question(&engine);

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert_eq!(
        question(&engine),
        asked,
        "the same question to the same seat, and no priority in between"
    );
    assert!(
        engine.resolution.is_some(),
        "the resolution is where it stopped"
    );
    assert_eq!(stack_size(&engine), 1, "Brainstorm, once");

    choose_the_first(&mut engine, seat(1));
    assert!(stack_is_empty(&engine));
    assert_eq!(
        hand_and_library(&engine, seat(1)),
        (cast.0 + 1, cast.1 - 1),
        "three drawn and two put back, once"
    );
    assert!(in_graveyard(&engine, seat(1), brainstorm()).is_some());
}

/// The chooser is Brainstorm's own caster. Their spell leaves the game with
/// them (CR 800.4a) and goes on resolving (CR 608.2m), but they choose
/// nothing (CR 800.4f), and what is left of it is putting cards from a hand
/// that left the game with them on top of a library that did too. Nobody
/// else draws, nothing is left in the resolution slot, and the active player
/// receives priority, as after any resolution (CR 117.3b).
#[test]
fn a_caster_who_leaves_mid_resolution_takes_their_part_with_them() {
    let (mut engine, _) = brainstorm_asking_seat_1(3);
    let others = [
        hand_and_library(&engine, seat(0)),
        hand_and_library(&engine, seat(2)),
    ];

    engine.apply(seat(1), PlayerAction::Concede).unwrap();
    assert!(engine.resolution.is_none(), "it has finished resolving");
    assert!(stack_is_empty(&engine), "Brainstorm left with its owner");
    assert_eq!(
        [
            hand_and_library(&engine, seat(0)),
            hand_and_library(&engine, seat(2)),
        ],
        others,
        "nobody else drew"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(0)),
        "{:?}",
        engine.pending()
    );
}

/// In a duel the player not being asked can still leave mid-resolution, and
/// the game is over at once (CR 104.2a), whatever was being asked.
#[test]
fn a_concession_mid_resolution_ends_a_duel() {
    let (mut engine, _) = brainstorm_asking_seat_1(2);

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    let Pending::GameOver(result) = engine.pending() else {
        panic!("the game went on: {:?}", engine.pending());
    };
    assert_eq!(result.winner, Some(Victor::Player(seat(1))));
}

/// Seat 2 leaves while seat 1 picks its sacrifice. Seat 1 is still asked,
/// the Edict finishes, and seat 2, whose creatures left the game with it,
/// is not asked at all.
#[test]
fn a_bystanders_concession_leaves_an_edict_with_the_seat_choosing() {
    let mut engine = edict_asking_seat_1();
    let asked = question(&engine);

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert_eq!(question(&engine), asked);

    choose_the_first(&mut engine, seat(1));
    assert!(stack_is_empty(&engine), "{:?}", engine.pending());
    assert_eq!(creatures_of(&engine, seat(1)).len(), 1);
    assert!(in_graveyard(&engine, seat(0), sheoldreds_edict()).is_some());
}

/// The Edict's caster leaves while seat 1 picks its sacrifice. The Edict
/// leaves the game with them and goes on resolving all the same (CR 608.2m):
/// seat 1 is still asked, and then seat 2, each an opponent of its caster by
/// its last known information (CR 800.4i).
#[test]
fn an_edict_whose_caster_leaves_still_resolves() {
    let mut engine = edict_asking_seat_1();
    let asked = question(&engine);

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert_eq!(question(&engine), asked);
    choose_the_first(&mut engine, seat(1));
    choose_the_first(&mut engine, seat(2));
    assert!(stack_is_empty(&engine), "{:?}", engine.pending());
    assert!(engine.resolution.is_none());
    assert_eq!(creatures_of(&engine, seat(1)).len(), 1);
    assert_eq!(creatures_of(&engine, seat(2)).len(), 1);
}

/// Seat 1 leaves while picking its sacrifice. Its choice goes with it
/// (CR 800.4a), and the same resolution goes on to seat 2, with no priority
/// in between. The Edict resolves once: seat 2 sacrifices one creature.
#[test]
fn an_edicts_chooser_who_leaves_takes_only_their_own_choice() {
    let mut engine = edict_asking_seat_1();

    engine.apply(seat(1), PlayerAction::Concede).unwrap();
    assert_eq!(
        engine.pending().asked(),
        Some(seat(2)),
        "{:?}",
        engine.pending()
    );
    assert_eq!(stack_size(&engine), 1, "the Edict is still resolving");

    choose_the_first(&mut engine, seat(2));
    assert!(stack_is_empty(&engine), "{:?}", engine.pending());
    assert_eq!(creatures_of(&engine, seat(2)).len(), 1);
    assert!(in_graveyard(&engine, seat(0), sheoldreds_edict()).is_some());
}

/// Seat 2 leaves while seat 0 is choosing Swift Spiral's target. The cast
/// goes on: the question stays with seat 0, and seat 2's creature, which
/// left the game with it, is no longer on offer and is refused. Once
/// cast, the spell is on the stack and seat 0 has priority (CR 601.2i).
/// Before #277 the cast was undone and priority went to seat 1.
#[test]
fn a_bystanders_concession_leaves_a_cast_without_their_creature() {
    let mut engine = swift_spiral_asking_for_a_target(&[1, 2]);
    let [ones] = creatures_of(&engine, seat(1))[..] else {
        panic!("one creature for seat 1");
    };
    let [theirs] = creatures_of(&engine, seat(2))[..] else {
        panic!("one creature for seat 2");
    };
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!("asserted by the helper")
    };
    assert!(options.contains(&ones) && options.contains(&theirs));

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the cast is still asking: {:?}", engine.pending());
    };
    assert_eq!(player, seat(0));
    assert_eq!(options, vec![ones]);
    assert!(
        engine
            .apply(
                seat(0),
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "the leaver's creature left the game with them"
    );

    engine
        .apply(
            seat(0),
            PlayerAction::ChooseObjects {
                objects: vec![ones],
            },
        )
        .unwrap();
    assert!(on_stack(&engine, twining_twins()).is_some());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(0)),
        "{:?}",
        engine.pending()
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "paid for");
}

/// The same, with a creature seat 1 owns and seat 2 controls, as if seat 2
/// had reanimated it (#281). It is exiled as seat 2 leaves (CR 800.4a) and
/// keeps its id in exile, so what drops it from the offer is that it moved,
/// not that it is gone.
#[test]
fn a_creature_exiled_as_its_controller_leaves_is_no_longer_on_offer() {
    let mut engine = Duel::table(277, plains(), 3)
        .hand(0, &[twining_twins()])
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    let [ones, lent] = creatures_of(&engine, seat(1))[..] else {
        panic!("two creatures for seat 1");
    };
    engine
        .dev_state_mut(seat(0))
        .expect("the harness sets boards up")
        .object_mut(lent)
        .expect("on the battlefield")
        .set_controller(seat(2));
    tap_all_mana(&mut engine, seat(0));
    let card = in_hand(&engine, seat(0), twining_twins()).expect("in hand");
    engine
        .apply(seat(0), PlayerAction::CastSpell { card })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("{:?}", engine.pending());
    };
    assert!(options.contains(&ones) && options.contains(&lent));

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the cast is still asking: {:?}", engine.pending());
    };
    assert_eq!((player, options), (seat(0), vec![ones]));
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(seat(1)))
            .contains(&lent)
    );
    assert!(
        engine
            .apply(
                seat(0),
                PlayerAction::ChooseObjects {
                    objects: vec![lent],
                },
            )
            .is_err()
    );
}

/// The only creature on offer was the leaver's. The cast can't go on, so
/// it is illegal, and the game returns to the moment before it was
/// proposed (CR 601.2): the card is back in hand, the stack is empty, and
/// seat 0 holds the priority it cast with. The mana it had floating before
/// the cast is still floating.
#[test]
fn a_cast_whose_only_target_left_the_game_is_reversed() {
    let mut engine = swift_spiral_asking_for_a_target(&[2]);

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert!(in_hand(&engine, seat(0), twining_twins()).is_some());
    assert!(stack_is_empty(&engine));
    assert!(engine.cast_wizard.is_none());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(0)),
        "{:?}",
        engine.pending()
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
}

/// A player who has left is no longer a player a spell can target.
#[test]
fn a_player_who_has_left_is_no_longer_a_target_on_offer() {
    let mut engine = Duel::table(277, island(), 3)
        .hand(0, &[ancestral_recall()])
        .battlefield(0, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    cast_from_hand(&mut engine, seat(0), ancestral_recall());
    let Pending::ChoosePlayer { options, .. } = engine.pending() else {
        panic!("expected the target, got {:?}", engine.pending());
    };
    assert!(options.contains(&seat(2)));

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    let Pending::ChoosePlayer { player, options } = engine.pending() else {
        panic!("the cast is still asking: {:?}", engine.pending());
    };
    assert_eq!(*player, seat(0));
    assert_eq!(options, &vec![seat(0), seat(1)]);
    assert!(
        engine
            .apply(seat(0), PlayerAction::ChoosePlayer(seat(2)))
            .is_err()
    );
    let library = hand_and_library(&engine, seat(1)).1;
    engine
        .apply(seat(0), PlayerAction::ChoosePlayer(seat(1)))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_and_library(&engine, seat(1)).1, library - 3);
}

/// A target chosen before its player left stays chosen, and is illegal
/// when the spell resolves: it is no longer in the zone it was in
/// (CR 608.2b). Swift Spiral does not resolve, so nothing is exiled or
/// waits for the end step, and the card goes to the graveyard rather than
/// on its adventure (CR 715.3d is what happens as it *resolves*).
#[test]
fn a_target_whose_player_left_is_illegal_on_resolution() {
    let mut engine = swift_spiral_asking_for_a_target(&[2]);
    let [theirs] = creatures_of(&engine, seat(2))[..] else {
        panic!("one creature for seat 2");
    };
    engine
        .apply(
            seat(0),
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .unwrap();
    assert!(on_stack(&engine, twining_twins()).is_some());

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, seat(0), twining_twins()).is_some());
    assert!(engine.state().delayed.is_empty(), "no return is waiting");
}

fn forest() -> CardIndex {
    basic_forest()
}

/// `{G}` 1/1, the spell Mana Leak is aimed at.
fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// `{1}{U}` "Counter target spell unless its controller pays {3}."
fn mana_leak() -> CardIndex {
    card_index("c61fe162-2202-4e56-9ba0-393547f9875f")
}

/// `{2}{B}{B}` "When this creature enters, destroy target creature an
/// opponent controls."
fn ravenous_chupacabra() -> CardIndex {
    card_index("7b459306-149b-4f43-abc1-2dd70c748c0e")
}

/// Its third line is "{2}{B}{B}, {T}, Sacrifice a Desert: Put two -1/-1
/// counters on target creature an opponent controls. Activate only as a
/// sorcery."
fn ifnir_deadlands() -> CardIndex {
    card_index("af698bd5-5f56-4d2a-9f02-8c3e781210cd")
}

/// "+2: Look at the top card of target player's library. You may put that
/// card on the bottom of that player's library."
fn jace_the_mind_sculptor() -> CardIndex {
    card_index("7f77a84e-5a4b-4834-aefa-3cecc175ae8e")
}

/// Taps the first mana source `seat` is offered.
fn tap_one(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    let source = *legal.mana_abilities.first().expect("a mana source");
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .unwrap();
}

/// Seat 0 casts Llanowar Elves off one of its four Forests, seat 1 answers
/// with Mana Leak, and seat 0 says it will pay the {3}. Nothing is floating,
/// so a payment window opens (CR 605.3a): seat 0 is asked to make the mana,
/// with the resolution suspended under the window.
fn mana_leak_window_open() -> Engine<RegistryLookup> {
    let mut engine = Duel::table(277, forest(), 3)
        .hand(0, &[llanowar_elves()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(1, &[mana_leak()])
        .battlefield(1, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    tap_one(&mut engine, seat(0));
    let elves = in_hand(&engine, seat(0), llanowar_elves()).expect("in hand");
    engine
        .apply(seat(0), PlayerAction::CastSpell { card: elves })
        .unwrap();
    engine.apply(seat(0), PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, seat(1), mana_leak());
    engine
        .apply(
            seat(1),
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.pending().asked(), Some(seat(0)));
    engine.apply(seat(0), PlayerAction::YesNo(true)).unwrap();
    assert!(engine.mana_window.is_some(), "{:?}", engine.pending());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(0)),
        "{:?}",
        engine.pending()
    );
    engine
}

/// A payment window is a resolution's question, whatever it looks like: a
/// bystander's concession leaves it open, asking the payer, and the payment
/// made in it counts.
#[test]
fn a_bystanders_concession_leaves_a_payment_window_open() {
    let mut engine = mana_leak_window_open();
    let asked = question(&engine);

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert_eq!(question(&engine), asked);
    assert!(engine.mana_window.is_some());

    tap_all_mana(&mut engine, seat(0));
    engine.apply(seat(0), PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        creatures_named(&engine, seat(0), llanowar_elves()) == 1,
        "the {{3}} was paid, so the Elves resolved"
    );
    assert!(in_graveyard(&engine, seat(1), mana_leak()).is_some());
}

/// The payer leaves inside the window. They pay nothing (CR 800.4f), and
/// Mana Leak finishes resolving at a spell that left the game with them.
/// Seat 0 was the active player, so the round goes on with seat 1
/// (CR 800.4j).
#[test]
fn a_payer_who_leaves_pays_nothing() {
    let mut engine = mana_leak_window_open();

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert!(engine.mana_window.is_none());
    assert!(engine.resolution.is_none());
    assert!(stack_is_empty(&engine));
    assert!(in_graveyard(&engine, seat(1), mana_leak()).is_some());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(1)),
        "{:?}",
        engine.pending()
    );
}

fn creatures_named(engine: &Engine<RegistryLookup>, player: PlayerId, card: CardIndex) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == player && o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

/// Ravenous Chupacabra enters for seat 0, and its trigger asks for a
/// creature an opponent controls: seat 2's is the only one. Seat 2 leaves,
/// and the trigger has no legal target, so it is removed from the stack
/// (CR 603.3d). The active player then has priority.
#[test]
fn a_trigger_whose_only_target_left_is_removed() {
    let mut engine = Duel::table(277, swamp(), 3)
        .hand(0, &[ravenous_chupacabra()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(2, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    cast_from_hand(&mut engine, seat(0), ravenous_chupacabra());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert!(engine.pending_plan.is_some(), "the trigger's own question");

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert!(engine.pending_plan.is_none());
    assert!(engine.trigger_queue.is_empty());
    assert!(stack_is_empty(&engine), "{:?}", engine.pending());
    assert_eq!(creatures_named(&engine, seat(0), ravenous_chupacabra()), 1);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(0)),
        "{:?}",
        engine.pending()
    );
}

/// Ifnir Deadlands' third line aims at a creature an opponent controls,
/// and seat 2's is the only one. Its targets are chosen before its costs
/// are paid (CR 601.2c before 601.2h, through CR 602.2b), so when seat 2
/// leaves, the activation is illegal and undone: the Deadlands is neither
/// tapped nor sacrificed, the black mana is still floating, and seat 0
/// still holds priority.
#[test]
fn an_activation_whose_only_target_left_is_reversed() {
    let mut engine = Duel::table(277, swamp(), 3)
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), ifnir_deadlands()])
        .battlefield(2, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    let deadlands = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == ifnir_deadlands())
        })
        .expect("the Deadlands is out");
    super::testkit::tap_mana_except(&mut engine, seat(0), deadlands);
    engine
        .apply(
            seat(0),
            PlayerAction::ActivateAbility {
                source: deadlands,
                ability_index: 2,
            },
        )
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "the target comes first: {:?}",
        engine.pending()
    );

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert!(engine.pending_plan.is_none());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(0)),
        "{:?}",
        engine.pending()
    );
    let land = engine
        .state()
        .object(deadlands)
        .expect("still on the battlefield");
    assert!(
        !land.status.contains(Status::TAPPED),
        "the {{T}} was never paid"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    assert!(stack_is_empty(&engine));
}

/// Seat 0 offers a draw and seat 1 is asked. Seat 1 leaves instead of
/// answering: a draw needs only the players still in the game (CR 104.4i),
/// so the offer goes on to seat 2, whose yes ends the game in a draw.
#[test]
fn a_draw_offer_goes_on_past_a_player_who_leaves_instead_of_answering() {
    let mut engine = Duel::table(277, forest(), 3).start();
    keep_mulligans(&mut engine);
    engine.apply(seat(0), PlayerAction::OfferDraw).unwrap();
    assert_eq!(engine.pending().asked(), Some(seat(1)));

    engine.apply(seat(1), PlayerAction::Concede).unwrap();
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, .. } if *player == seat(2)),
        "{:?}",
        engine.pending()
    );
    engine.apply(seat(2), PlayerAction::YesNo(true)).unwrap();
    let Pending::GameOver(result) = engine.pending() else {
        panic!("the game went on: {:?}", engine.pending());
    };
    assert_eq!(result.winner, None);
}

/// Seat 2 leaves while seat 1 is deciding on seat 0's draw offer. Seat 1 is
/// still the one asked, and once it agrees nobody is left to ask: seat 2 is
/// no longer one of the players.
#[test]
fn a_draw_offer_does_not_wait_on_a_player_who_has_left() {
    let mut engine = Duel::table(277, forest(), 3).start();
    keep_mulligans(&mut engine);
    engine.apply(seat(0), PlayerAction::OfferDraw).unwrap();
    let asked = question(&engine);

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert_eq!(question(&engine), asked);
    engine.apply(seat(1), PlayerAction::YesNo(true)).unwrap();
    let Pending::GameOver(result) = engine.pending() else {
        panic!("the offer still waits: {:?}", engine.pending());
    };
    assert_eq!(result.winner, None);
}

/// Jace's +2 looks at the top card of seat 2's library, and seat 2 leaves
/// while seat 0 decides where it goes. The card left the game with its
/// owner, so there is nothing left to place, and the question asks for
/// nothing: every pile may be empty.
#[test]
fn a_look_at_a_departed_players_library_has_nothing_left_to_place() {
    let mut engine = Duel::table(277, island(), 3)
        .battlefield(0, &[jace_the_mind_sculptor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    let jace = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    engine
        .apply(
            seat(0),
            PlayerAction::ActivateAbility {
                source: jace,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(seat(0), PlayerAction::ChoosePlayer(seat(2)))
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, piles, .. } = engine.pending().clone() else {
        unreachable!("waited for above")
    };
    assert_eq!(cards.len(), 1, "seat 2's top card");

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    let Pending::Arrange {
        player,
        cards,
        piles: now,
        ..
    } = engine.pending().clone()
    else {
        panic!("the look is still asking: {:?}", engine.pending());
    };
    assert_eq!(player, seat(0));
    assert!(cards.is_empty());
    assert!(now.iter().all(|pile| pile.min == 0));
    engine
        .apply(
            seat(0),
            PlayerAction::Arrange {
                piles: vec![Vec::new(); piles.len()],
            },
        )
        .unwrap();
    assert!(stack_is_empty(&engine), "{:?}", engine.pending());
}

/// Nobody receives priority inside a payment window, so nothing checks
/// state-based actions there (CR 704.3), and a bystander's concession
/// leaves it that way. Seat 1 at zero life has not lost while seat 0 is
/// still making the mana. Asking seat 0 again through the round would have
/// run them.
#[test]
fn a_concession_inside_a_payment_window_checks_no_state_based_actions() {
    let mut engine = mana_leak_window_open();
    let asked = question(&engine);
    engine.state.players[1].life = 0;

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert_eq!(question(&engine), asked);
    assert!(!engine.state().players[1].has_lost());
}

/// Seat 0's own Chupacabra trigger waits on seat 0's target, and seat 0
/// leaves. The trigger is never put on the stack (CR 800.4d), nobody is
/// asked on its behalf, and seat 1's creature stays.
#[test]
fn a_leavers_trigger_waiting_on_its_target_goes_with_them() {
    let mut engine = Duel::table(277, swamp(), 3)
        .hand(0, &[ravenous_chupacabra()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    cast_from_hand(&mut engine, seat(0), ravenous_chupacabra());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(engine.pending().asked(), Some(seat(0)));

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert_ne!(
        engine.pending().asked(),
        Some(seat(0)),
        "{:?}",
        engine.pending()
    );
    assert!(engine.trigger_queue.is_empty());
    assert!(stack_is_empty(&engine));
    assert_eq!(creatures_of(&engine, seat(1)).len(), 1);
}

/// No card in the pool asks a player to sort somebody else's cards into a
/// pile that has to take some (Jace's look lets every pile be empty), so
/// the question is built by hand. Seat 2's card is taken out, and the piles
/// ask for no more than the one card left: the first still takes it, and
/// the second may now be empty.
#[test]
fn piles_ask_for_no_more_cards_than_are_left() {
    use crate::choice::{ArrangePile, ArrangePlace, ArrangePrompt};
    let mut engine = Duel::table(277, island(), 3).start();
    keep_mulligans(&mut engine);
    let top = |p: u8| engine.state().zones.list(ZoneLocation::Library(seat(p)))[0];
    let (mine, theirs) = (top(1), top(2));
    engine.pending = Pending::Arrange {
        player: seat(0),
        cards: vec![mine, theirs],
        piles: vec![
            ArrangePile::all_of(ArrangePlace::LibraryBottom, 1),
            ArrangePile::all_of(ArrangePlace::LibraryTop, 1),
        ],
        prompt: ArrangePrompt::Order,
    };
    sba::eliminate_player(
        &mut engine.state,
        seat(2),
        crate::event::LossReason::Conceded,
    );

    assert!(engine.forget_the_departed(&[]));
    let Pending::Arrange { cards, piles, .. } = engine.pending() else {
        unreachable!("set above")
    };
    assert_eq!(cards, &vec![mine]);
    assert_eq!(
        piles.iter().map(|p| (p.min, p.max)).collect::<Vec<_>>(),
        vec![(1, 1), (0, 1)]
    );
}

/// Four seats. Seat 0 passes, and seat 1 offers a draw, which seat 0
/// accepts. Seat 0 leaves while seat 2 is deciding, and seat 2 refuses,
/// which hands seat 1 its priority back as it was. Seat 0's pass is not one
/// by a player still in the game (CR 117.4): the step waits for seats 1, 2
/// and 3.
#[test]
fn a_leavers_pass_does_not_count_when_the_round_resumes() {
    let mut engine = Duel::table(277, forest(), 4).start();
    keep_mulligans(&mut engine);
    let step = engine.state().turn.step;
    engine.apply(seat(0), PlayerAction::PassPriority).unwrap();
    engine.apply(seat(1), PlayerAction::OfferDraw).unwrap();
    engine.apply(seat(0), PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.pending().asked(), Some(seat(2)));
    assert_eq!(engine.passes, 1);

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    engine.apply(seat(2), PlayerAction::YesNo(false)).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(1)),
        "{:?}",
        engine.pending()
    );
    assert_eq!(engine.passes, 0, "seat 0's pass is gone with it");
    engine.apply(seat(1), PlayerAction::PassPriority).unwrap();
    engine.apply(seat(2), PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.pending().asked(),
        Some(seat(3)),
        "the step ended before seat 3 passed: {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().turn.step, step);
}

/// `{U}{U}` "Counter target spell. At the beginning of your next main
/// phase, add an amount of {C} equal to that spell's mana value."
fn mana_drain() -> CardIndex {
    card_index("74d3277a-38e5-4732-afed-084a56148f20")
}

/// `{W}` "Whenever another creature enters, you gain 1 life."
fn soul_warden() -> CardIndex {
    card_index("f3fad295-1af2-4ecc-8546-b121ad6be27b")
}

/// Swift Spiral resolves on seat 1's creature, and its caster leaves before
/// the end step. The return is a delayed trigger seat 0 controls
/// (CR 603.7d), and one controlled by a player who has left isn't put on
/// the stack (CR 800.4d): the creature stays in exile, and nothing is left
/// waiting for a turn that seat 0 will never have.
#[test]
fn a_departed_casters_end_step_return_does_not_happen() {
    let mut engine = swift_spiral_asking_for_a_target(&[1]);
    let [theirs] = creatures_of(&engine, seat(1))[..] else {
        panic!("one creature for seat 1");
    };
    engine
        .apply(
            seat(0),
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(creatures_of(&engine, seat(1)).is_empty());
    assert_eq!(engine.state().delayed.len(), 1, "the return is waiting");

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert!(
        engine.state().delayed.is_empty(),
        "{:?}",
        engine.state().delayed
    );
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number != turn);
    assert!(
        creatures_of(&engine, seat(1)).is_empty(),
        "the creature came back"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(seat(1)))
            .len(),
        1
    );
}

/// Seat 1 casts Brainstorm in seat 0's upkeep, and seat 0 Mana Drains it.
/// Seat 0 leaves before its main phase, and nobody gets the mana
/// (CR 800.4d).
#[test]
fn a_departed_players_mana_drain_adds_nothing() {
    let mut engine = Duel::table(278, island(), 3)
        .hand(0, &[mana_drain()])
        .battlefield(0, &[island(), island()])
        .hand(1, &[brainstorm()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    engine.apply(seat(0), PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, seat(1), brainstorm());
    pass_until(&mut engine, |e| {
        e.pending().asked() == Some(seat(0)) && !stack_is_empty(e)
    });
    let spell = on_stack(&engine, brainstorm()).expect("on the stack");
    cast_from_hand(&mut engine, seat(0), mana_drain());
    engine
        .apply(
            seat(0),
            PlayerAction::ChooseTargets {
                objects: vec![spell],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().turn.step, Step::Upkeep);
    assert_eq!(engine.state().delayed.len(), 1, "the mana is waiting");

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    pass_until(&mut engine, |e| e.state().turn.step == Step::Main);
    assert_eq!(engine.state().turn.number, 1);
    let pools: Vec<u32> = engine
        .state()
        .players
        .iter()
        .map(|p| p.mana_pool.total())
        .collect();
    assert_eq!(pools, [0, 0, 0]);
}

/// Ravenous Chupacabra enters under seat 0, and seat 1's Soul Warden
/// triggers on it as well. Seat 0's trigger is put on the stack first
/// (CR 603.3b) and asks for its target, and seat 1 leaves while it does.
/// Their trigger has triggered and is waiting for the stack, and it never
/// gets there (CR 800.4d).
#[test]
fn a_trigger_waiting_for_the_stack_goes_with_its_controller() {
    let mut engine = Duel::table(278, swamp(), 3)
        .hand(0, &[ravenous_chupacabra()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[soul_warden()])
        .battlefield(2, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    cast_from_hand(&mut engine, seat(0), ravenous_chupacabra());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let controllers: Vec<PlayerId> = engine.trigger_queue.iter().map(|t| t.controller).collect();
    assert_eq!(controllers, [seat(0), seat(1)]);

    engine.apply(seat(1), PlayerAction::Concede).unwrap();
    let [theirs] = creatures_of(&engine, seat(2))[..] else {
        panic!("one creature for seat 2");
    };
    engine
        .apply(
            seat(0),
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .unwrap();
    assert!(engine.trigger_queue.is_empty());
    assert_eq!(stack_size(&engine), 1, "Chupacabra's trigger alone");
    let before = engine.state().players[1].life;
    pass_until(&mut engine, stack_is_empty);
    assert!(creatures_of(&engine, seat(2)).is_empty());
    assert_eq!(engine.state().players[1].life, before);
}

/// The door is where a delayed action is performed, not only the list it
/// waits in: one that had already come due when its controller left is
/// not performed either (CR 800.4d).
#[test]
fn a_delayed_action_come_due_is_not_performed_for_a_player_who_has_left() {
    let mut engine = Duel::table(278, plains(), 3)
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    let [theirs] = creatures_of(&engine, seat(1))[..] else {
        panic!("one creature for seat 1");
    };
    engine
        .state
        .move_object(
            theirs,
            ZoneLocation::Exile(seat(1)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let card = engine.state().zones.list(ZoneLocation::Exile(seat(1)))[0];
    engine.delayed_queue.push_back((
        seat(0),
        crate::state::DelayedAction::ReturnToBattlefield { card },
    ));
    sba::eliminate_player(
        &mut engine.state,
        seat(0),
        crate::event::LossReason::Conceded,
    );

    assert!(!engine.process_delayed());
    assert!(engine.delayed_queue.is_empty());
    assert!(creatures_of(&engine, seat(1)).is_empty());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(seat(1)))[..],
        [card]
    );
}

/// Mana a delayed trigger adds goes to the player who controls it
/// (CR 603.7d), whoever is active.
#[test]
fn delayed_mana_goes_to_its_controller() {
    let mut engine = Duel::table(278, plains(), 3).start();
    keep_mulligans(&mut engine);
    assert_eq!(engine.state().turn.active, seat(0));
    engine.delayed_queue.push_back((
        seat(2),
        crate::state::DelayedAction::AddMana {
            color: baylee_core::mana::ManaColor::Colorless,
            amount: 2,
        },
    ));

    assert!(!engine.process_delayed());
    let pools: Vec<u32> = engine
        .state()
        .players
        .iter()
        .map(|p| p.mana_pool.total())
        .collect();
    assert_eq!(pools, [0, 0, 2]);
}
