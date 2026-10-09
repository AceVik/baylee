//! Word of Command: "The player plays that card if able. While doing so, the
//! player can activate mana abilities only if they're from lands that player
//! controls and only if mana they produce is spent to activate other mana
//! abilities of lands the player controls and/or to play that card."
//!
//! A casting that cannot comply with a step is reversed (CR 601.2, CR 732.1),
//! and a player able to play the card is not free to give it up. These tests
//! hold the exact feasibility of the commanded payment: a card its lands
//! cannot pay is not played and nothing is tapped; a payment that can still
//! be made cannot be abandoned; and a choice that would leave it unpayable
//! is refused while the right one is taken.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const USER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);
const MINOTAUR: CardIndex = index::HURLOON_MINOTAUR;

fn fixture(board: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[index::WORD_OF_COMMAND])
        .battlefield(1, board)
        .hand(1, &[MINOTAUR])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    engine
}

/// Casts Word at the opponent and chooses the Minotaur from their hand.
fn command_minotaur(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    for source in all_on_battlefield(engine, USER, swamp()) {
        engine
            .apply(USER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    cast_with_floating(engine, USER, index::WORD_OF_COMMAND);
    match engine.pending() {
        Pending::ChoosePlayer { .. } => {
            engine
                .apply(USER, PlayerAction::ChoosePlayer(OTHER))
                .unwrap();
        }
        _ => aim(engine, vec![], vec![OTHER]),
    }
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let minotaur = in_hand(engine, OTHER, MINOTAUR).unwrap();
    engine
        .apply(
            USER,
            PlayerAction::ChooseObjects {
                objects: vec![minotaur],
            },
        )
        .unwrap();
    minotaur
}

fn untapped(engine: &Engine<RegistryLookup>, card: CardIndex) -> Vec<ObjectId> {
    all_on_battlefield(engine, OTHER, card)
        .into_iter()
        .filter(|id| !is_tapped(engine, *id))
        .collect()
}

fn in_commanded_window(engine: &Engine<RegistryLookup>) -> bool {
    engine
        .payment_window()
        .is_some_and(|(player, _)| player == OTHER)
        && engine.decision_actor() == Some(USER)
}

/// {1}{R}{R} from a Mountain and two Islands cannot be paid: the card is not
/// played, nothing is tapped, and Word finishes. Before the exact check, the
/// controller could tap the Mountain and an Island and then had no answer
/// left: every further tap and the pass were refused.
#[test]
fn an_unpayable_commanded_card_is_not_played_and_nothing_is_tapped() {
    let mut engine = fixture(&[mountain(), island(), island()]);
    let minotaur = command_minotaur(&mut engine);
    assert!(
        !in_commanded_window(&engine),
        "no payment is opened for a price the lands cannot pay"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(minotaur).unwrap().zone, Zone::Hand);
    assert_eq!(untapped(&engine, mountain()).len(), 1);
    assert_eq!(untapped(&engine, island()).len(), 2);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert!(in_graveyard(&engine, USER, index::WORD_OF_COMMAND).is_some());
    assert!(
        !engine.may_inspect_private(USER, OTHER),
        "control has ended"
    );
}

/// {1}{R}{R} from two Mountains and an Island can be paid, so the player is
/// able to play it: passing the window early is refused and changes nothing,
/// and the paid card is cast under its owner's control.
#[test]
fn a_payable_commanded_card_cannot_be_abandoned() {
    let mut engine = fixture(&[mountain(), mountain(), island()]);
    let minotaur = command_minotaur(&mut engine);
    assert!(in_commanded_window(&engine));
    let before = engine.fingerprint();
    assert!(
        engine.apply(USER, PlayerAction::PassPriority).is_err(),
        "giving up a card the player can pay for is refused"
    );
    assert_eq!(engine.fingerprint(), before, "and leaves nothing behind");

    let source = untapped(&engine, mountain())[0];
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source })
        .unwrap();
    assert!(
        engine.apply(USER, PlayerAction::PassPriority).is_err(),
        "one red of three is still not a payment"
    );
    for source in untapped(&engine, mountain())
        .into_iter()
        .chain(untapped(&engine, island()))
    {
        engine
            .apply(USER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.state().object(minotaur).unwrap().zone, Zone::Stack);
    pass_until(&mut engine, stack_is_empty);
    let minotaur = on_battlefield(&engine, OTHER, MINOTAUR).expect("it resolved");
    assert_eq!(engine.state().object(minotaur).unwrap().controller, OTHER);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
}

/// Mountain, Plateau and Island pay {1}{R}{R} only with the Plateau's red.
/// Its white would leave a unit the price cannot use and the price short of
/// red, which the old optimistic check accepted (it imagined more lands). It
/// is refused as the colour is chosen, and red is taken.
#[test]
fn a_colour_that_would_leave_the_payment_unpayable_is_refused() {
    let mut engine = fixture(&[mountain(), index::PLATEAU, island()]);
    command_minotaur(&mut engine);
    assert!(in_commanded_window(&engine));
    let plateau = on_battlefield(&engine, OTHER, index::PLATEAU).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("payment window expected, got {:?}", engine.pending())
    };
    let action = legal
        .abilities
        .iter()
        .find(|(source, _)| *source == plateau)
        .map(|&(source, ability_index)| PlayerAction::ActivateAbility {
            source,
            ability_index,
        })
        .unwrap_or(PlayerAction::ActivateManaAbility { source: plateau });
    engine.apply(USER, action).unwrap();
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "the Plateau asks for its colour, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, OTHER);
    assert!(options.contains(&ManaColor::White) && options.contains(&ManaColor::Red));
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(USER, PlayerAction::ChooseColor(ManaColor::White))
            .is_err(),
        "white leaves {{1}}{{R}}{{R}} unpayable from a Mountain and an Island"
    );
    assert_eq!(engine.fingerprint(), before);
    engine
        .apply(USER, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    for source in untapped(&engine, mountain())
        .into_iter()
        .chain(untapped(&engine, island()))
    {
        engine
            .apply(USER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, OTHER, MINOTAUR).is_some());
}

/// Mountain, Mountain, Island, Island: one Island's blue fits the {1}, a
/// second has nowhere to go, and its tap is refused; the payment then
/// completes with both Mountains. (The older optimistic proof caught this
/// one too; it stays as the exact check's regression.)
#[test]
fn a_second_island_that_the_price_cannot_use_is_refused() {
    let mut engine = fixture(&[mountain(), mountain(), island(), island()]);
    command_minotaur(&mut engine);
    let islands = untapped(&engine, island());
    engine
        .apply(
            USER,
            PlayerAction::ActivateManaAbility { source: islands[0] },
        )
        .unwrap();
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(
                USER,
                PlayerAction::ActivateManaAbility { source: islands[1] }
            )
            .is_err()
    );
    assert_eq!(engine.fingerprint(), before);
    for source in untapped(&engine, mountain()) {
        engine
            .apply(USER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, OTHER, MINOTAUR).is_some());
    assert_eq!(
        untapped(&engine, island()).len(),
        1,
        "one Island never tapped"
    );
}

/// {1}{R}{R} from a Mountain, two Islands and an Unknown Shores ("{1}, {T}:
/// Add one mana of any color"): the Mountain's red, an Island's blue paying
/// the Shores for a second red, the other Island's blue for the {1}. The
/// Shores' mana ability costs mana, which the exact reader used to give up
/// on; it now finds the payment, so the player is able to play the card and
/// passing the window is refused (it used to be allowed, and the card was
/// lost).
#[test]
fn a_land_whose_mana_ability_costs_mana_is_read_exactly_when_it_pays() {
    let mut engine = fixture(&[mountain(), island(), island(), index::UNKNOWN_SHORES]);
    command_minotaur(&mut engine);
    assert!(in_commanded_window(&engine));
    assert!(
        engine.apply(USER, PlayerAction::PassPriority).is_err(),
        "the Shores make the price payable, so the card cannot be given up"
    );
}

/// {1}{R}{R} from a Mountain, an Island and an Unknown Shores: two reds at
/// most (the Island paying the Shores), and nothing left for the {1}. The
/// card is not played and nothing is tapped; before, the costed Shores sent
/// the check to the optimistic proof, which opened the payment.
#[test]
fn a_land_whose_mana_ability_costs_mana_is_read_exactly_when_it_cannot_pay() {
    let mut engine = fixture(&[mountain(), island(), index::UNKNOWN_SHORES]);
    let minotaur = command_minotaur(&mut engine);
    assert!(
        !in_commanded_window(&engine),
        "no payment is opened for a price the lands cannot pay"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(minotaur).unwrap().zone, Zone::Hand);
    assert_eq!(untapped(&engine, mountain()).len(), 1);
    assert_eq!(untapped(&engine, island()).len(), 1);
    assert_eq!(untapped(&engine, index::UNKNOWN_SHORES).len(), 1);
}

/// "Plays that card if able": Brazen Borrower ({1}{U}{U}) cannot be paid
/// from an Island and a Mountain, its adventure Petty Theft ({1}{U}) can. The
/// way of casting the lands cannot pay is not offered, so the controller is
/// never asked to choose a mode that would then be reversed while the other
/// would have been played: only Petty Theft is cast, at the controller's
/// creature.
#[test]
fn a_way_of_casting_the_lands_cannot_pay_is_not_offered() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), index::GRIZZLY_BEARS])
        .hand(0, &[index::WORD_OF_COMMAND])
        .battlefield(1, &[island(), mountain()])
        .hand(1, &[index::BRAZEN_BORROWER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    for source in all_on_battlefield(&engine, USER, swamp()) {
        engine
            .apply(USER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    cast_with_floating(&mut engine, USER, index::WORD_OF_COMMAND);
    match engine.pending() {
        Pending::ChoosePlayer { .. } => {
            engine
                .apply(USER, PlayerAction::ChoosePlayer(OTHER))
                .unwrap();
        }
        _ => aim(&mut engine, vec![], vec![OTHER]),
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let borrower = in_hand(&engine, OTHER, index::BRAZEN_BORROWER).unwrap();
    engine
        .apply(
            USER,
            PlayerAction::ChooseObjects {
                objects: vec![borrower],
            },
        )
        .unwrap();
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        assert!(
            options
                .iter()
                .all(|o| matches!(o.kind, crate::choice::CastModeKind::Face(1))),
            "the creature's price is beyond the lands: {options:?}"
        );
        engine.apply(USER, PlayerAction::ChooseMode(0)).unwrap();
    }
    let bears = on_battlefield(&engine, USER, index::GRIZZLY_BEARS).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Petty Theft asks its target, got {:?}", engine.pending());
    };
    assert!(options.contains(&bears));
}
