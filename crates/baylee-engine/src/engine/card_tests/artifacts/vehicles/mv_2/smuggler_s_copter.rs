//! `cards/artifacts/vehicles/mv_2/smuggler_s_copter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crew lasts one turn. Both attacking and blocking trigger optional draw/discard.
#[test]
fn smugglers_copter_crews_and_loots_on_attack_or_block() {
    use crate::engine::card_tests::legends_kit as kit;
    for blocking in [false, true] {
        let mut e = kit::game(
            980,
            &[smugglers_copter(), llanowar_elves()],
            &[serra_angel()],
            &[forest()],
            &[],
        );
        let copter = kit::obj(&e, kit::P0, smugglers_copter());
        let elf = kit::obj(&e, kit::P0, llanowar_elves());
        if blocking {
            pass_until(&mut e, |e| {
                e.state().turn.active == kit::P1
                    && matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P0)
            });
        }
        assert!(!types(&e, copter).contains(TypeSet::CREATURE));
        activate(&mut e, kit::P0, smugglers_copter(), 2);
        e.apply(kit::P0, PlayerAction::ChooseObjects { objects: vec![elf] })
            .unwrap();
        assert!(is_tapped(&e, elf));
        pass_until(&mut e, stack_is_empty);
        assert!(types(&e, copter).contains(TypeSet::CREATURE));
        assert_eq!(pt(&e, copter), (3, 3));
        let before = library_size(&e, kit::P0);
        if blocking {
            let angel = kit::obj(&e, kit::P1, serra_angel());
            kit::attack(&mut e, &[angel]);
            kit::block(&mut e, &[(copter, angel)]);
        } else {
            kit::attackers_offered(&mut e);
            e.apply(
                kit::P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(copter, Defender::Player(kit::P1))],
                },
            )
            .unwrap();
        }
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        kit::yes_no(&mut e, true);
        assert_eq!(library_size(&e, kit::P0), before - 1);
        let Pending::ChooseCards { options, .. } = e.pending().clone() else {
            panic!("discard after draw")
        };
        e.apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
    }
}

/// A game stopped at the Copter's "you may draw a card" question: crewed by
/// the Elves, attacking (or blocking the Angel on p1's turn).
fn copter_at_its_loot_question(blocking: bool) -> Engine<RegistryLookup> {
    use crate::engine::card_tests::legends_kit as kit;
    let mut e = kit::game(
        980,
        &[smugglers_copter(), llanowar_elves()],
        &[serra_angel()],
        &[forest()],
        &[],
    );
    let copter = kit::obj(&e, kit::P0, smugglers_copter());
    let elf = kit::obj(&e, kit::P0, llanowar_elves());
    if blocking {
        pass_until(&mut e, |e| {
            e.state().turn.active == kit::P1
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P0)
        });
    }
    activate(&mut e, kit::P0, smugglers_copter(), 2);
    e.apply(kit::P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    if blocking {
        let angel = kit::obj(&e, kit::P1, serra_angel());
        kit::attack(&mut e, &[angel]);
        kit::block(&mut e, &[(copter, angel)]);
    } else {
        kit::attackers_offered(&mut e);
        e.apply(
            kit::P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(copter, Defender::Player(kit::P1))],
            },
        )
        .unwrap();
    }
    // Not `pass_until` past the question: it answers "you may" with yes.
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e
}

fn hand_and_graveyard(e: &Engine<RegistryLookup>) -> (usize, usize) {
    let seat = PlayerId::new(0);
    (
        e.state().zones.list(ZoneLocation::Hand(seat)).len(),
        e.state().zones.list(ZoneLocation::Graveyard(seat)).len(),
    )
}

/// "you **may** draw a card. If you do, discard a card." Declined, on the
/// attack and on the block: nothing is drawn, no discard is asked and the
/// hand and graveyard are as they were.
#[test]
fn smugglers_copter_declined_loots_nothing() {
    let p0 = PlayerId::new(0);
    for blocking in [false, true] {
        let mut e = copter_at_its_loot_question(blocking);
        let library = library_size(&e, p0);
        let (hand, graveyard) = hand_and_graveyard(&e);
        e.apply(p0, PlayerAction::YesNo(false)).unwrap();
        assert!(
            !matches!(e.pending(), Pending::ChooseCards { .. }),
            "declined, so there is nothing to discard (blocking = {blocking}): {:?}",
            e.pending()
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            library_size(&e, p0),
            library,
            "no card was drawn (blocking = {blocking})"
        );
        assert_eq!(
            hand_and_graveyard(&e),
            (hand, graveyard),
            "and nothing was discarded (blocking = {blocking})"
        );
    }
}

/// The accepted loot is draw one, **then discard one**: the hand ends the
/// size it began, the library is one card shorter and the graveyard holds
/// exactly the card that was discarded.
#[test]
fn smugglers_copter_accepted_draws_then_discards_one() {
    let p0 = PlayerId::new(0);
    for blocking in [false, true] {
        let mut e = copter_at_its_loot_question(blocking);
        let library = library_size(&e, p0);
        let (hand, graveyard) = hand_and_graveyard(&e);
        e.apply(p0, PlayerAction::YesNo(true)).unwrap();
        assert_eq!(
            library_size(&e, p0),
            library - 1,
            "one card drawn (blocking = {blocking})"
        );
        let Pending::ChooseCards { options, .. } = e.pending().clone() else {
            panic!(
                "the discard is asked right after the draw: {:?}",
                e.pending()
            )
        };
        assert_eq!(
            options.len(),
            hand + 1,
            "the whole hand, the drawn card included, is on offer (blocking = {blocking})"
        );
        let discarded = options[0];
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discarded],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            hand_and_graveyard(&e),
            (hand, graveyard + 1),
            "drew one and discarded one: the hand is unchanged, the graveyard a card longer \
             (blocking = {blocking})"
        );
        assert_eq!(
            e.state().object(discarded).unwrap().zone,
            Zone::Graveyard,
            "and the card chosen is the card in it (blocking = {blocking})"
        );
    }
}
