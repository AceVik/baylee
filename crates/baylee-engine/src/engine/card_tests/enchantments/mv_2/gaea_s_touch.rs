//! `cards/enchantments/mv_2/gaea_s_touch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn touch_card() -> CardIndex {
    card_index("ddd053ea-5b91-4def-a003-2e297a170d99")
}

fn game(hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, swamp())
        .hand(0, hand)
        .battlefield(0, &[touch_card()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn touch_offered(e: &Engine<RegistryLookup>) -> bool {
    let touch = on_battlefield(e, P0, touch_card()).expect("Gaea's Touch is out");
    priority_offer(e).abilities.contains(&(touch, 0))
}

/// Gaea's Touch (ddd053ea-5b91-4def-a003-2e297a170d99) — "{0}: You may put a
/// basic Forest card from your hand onto the battlefield. Activate only as a
/// sorcery and only once each turn." Only the Forest is on the menu.
#[test]
fn gaea_s_touch_puts_a_basic_forest_from_hand_onto_the_battlefield() {
    let mut e = game(&[forest(), island()]);
    let hand_forest = in_hand(&e, P0, forest()).expect("a Forest in hand");
    let hand_island = in_hand(&e, P0, island()).expect("an Island in hand");
    assert!(touch_offered(&e), "free, and it is our main phase");

    activate(&mut e, P0, touch_card(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!("pass_until stopped on the choice")
    };
    assert_eq!(player, P0);
    assert!(options.contains(&hand_forest), "the Forest is offered");
    assert!(!options.contains(&hand_island), "the Island is not");
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![hand_forest],
        },
    )
    .expect("the Forest is a legal pick");
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().object(hand_forest).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the Forest came into play"
    );
    assert!(in_hand(&e, P0, island()).is_some(), "the Island stayed");
}

/// Gaea's Touch — "You may": declining leaves the Forest in hand.
#[test]
fn gaea_s_touch_may_decline_to_put_anything() {
    let mut e = game(&[forest()]);
    let hand_forest = in_hand(&e, P0, forest()).expect("a Forest in hand");
    activate(&mut e, P0, touch_card(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("putting none is allowed");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(hand_forest).map(|o| o.zone),
        Some(Zone::Hand),
        "still in hand"
    );
}

/// Gaea's Touch — "only once each turn": a second activation in the same turn
/// is not offered, and it is offered again on the next turn of its controller.
#[test]
fn gaea_s_touch_can_be_activated_only_once_each_turn() {
    let mut e = game(&[forest(), forest()]);
    activate(&mut e, P0, touch_card(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!("pass_until stopped on the choice")
    };
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![options[0]],
        },
    )
    .expect("a Forest is a legal pick");
    pass_until(&mut e, stack_is_empty);
    assert!(
        !touch_offered(&e),
        "once each turn: the second activation is not offered"
    );

    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    assert!(touch_offered(&e), "a new turn, a new activation");
}

/// Gaea's Touch — "Activate only as a sorcery": not at the beginning of the
/// turn's upkeep, even with priority.
#[test]
fn gaea_s_touch_is_not_offered_outside_a_main_phase() {
    let mut e = game(&[forest()]);
    reach_their_main_phase(&mut e, P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(!touch_offered(&e), "an upkeep is no time for a sorcery");
}

/// Gaea's Touch — "Sacrifice this enchantment: Add {G}{G}."
#[test]
fn gaea_s_touch_sacrifices_itself_for_two_green_mana() {
    let mut e = game(&[]);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Green), 0);
    activate(&mut e, P0, touch_card(), 1);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Green),
        2,
        "{{G}}{{G}} in the pool"
    );
    assert!(
        on_battlefield(&e, P0, touch_card()).is_none(),
        "it was sacrificed as the cost"
    );
    assert!(in_graveyard(&e, P0, touch_card()).is_some());
}
