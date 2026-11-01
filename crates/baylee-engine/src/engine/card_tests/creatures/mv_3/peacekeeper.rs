//! `cards/creatures/mv_3/peacekeeper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn peacekeeper() -> CardIndex {
    card_index("e82ce5c6-1e42-4e87-b696-67aa3822ec9c")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Peacekeeper and a Grizzly Bears on player 0's side with two Plains,
/// a Grizzly Bears on player 1's, run to the first question of player 0's
/// upkeep.
fn table() -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[peacekeeper(), grizzly_bears(), plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e
}

/// Says yes to the upkeep payment and pays `{1}{W}` with both Plains.
#[track_caller]
fn pay_with_both_plains(e: &mut Engine<RegistryLookup>) {
    let lands: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.controller == P0 && o.card.is_some_and(|c| c.index == plains()))
        })
        .collect();
    assert_eq!(lands.len(), 2, "two Plains to pay with");
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    e.apply(
        P0,
        PlayerAction::PayWindow {
            steps: lands
                .into_iter()
                .map(|source| PlayerAction::ActivateManaAbility { source })
                .collect(),
        },
    )
    .expect("the Plains pay");
}

/// The attackers offered by the next declare-attackers question.
fn attackers_in_next_declaration(e: &mut Engine<RegistryLookup>) -> (PlayerId, Vec<ObjectId>) {
    pass_until(e, |e| e.state().turn.step == Step::DeclareAttackers);
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = e.pending().clone()
    else {
        panic!("expected the declare-attackers question: {:?}", e.pending())
    };
    (player, attackers)
}

/// Peacekeeper (e82ce5c6-1e42-4e87-b696-67aa3822ec9c) — "At the beginning of
/// your upkeep, sacrifice this creature unless you pay {1}{W}. Creatures can't
/// attack." Paid with both Plains: it stays, and neither player's Bears may
/// attack — not its controller's on this turn, not the opponent's on theirs —
/// and the opponent's upkeep asks nothing.
#[test]
fn peacekeeper_when_paid_stops_every_creature_from_attacking() {
    let mut e = table();
    pay_with_both_plains(&mut e);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, peacekeeper()).is_some(),
        "paid, so it stays"
    );

    let (player, attackers) = attackers_in_next_declaration(&mut e);
    assert_eq!(player, P0);
    assert!(
        attackers.is_empty(),
        "no creature of its controller may attack: {attackers:?}"
    );

    e.apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::DeclareAttackers
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = e.pending().clone()
    else {
        panic!(
            "expected player 1's declare-attackers question: {:?}",
            e.pending()
        )
    };
    assert_eq!(player, P1);
    assert!(attackers.is_empty(), "nor the opponent's: {attackers:?}");
    assert!(
        on_battlefield(&e, P0, peacekeeper()).is_some(),
        "the opponent's upkeep did not tax it"
    );
}

/// Peacekeeper — "unless you pay": declined, it is sacrificed and with it goes
/// the attack ban, the Bears is offered as an attacker on that very turn.
#[test]
fn peacekeeper_declined_is_sacrificed_and_attacks_are_allowed_again() {
    let mut e = table();
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P0, peacekeeper()).is_some(),
        "unpaid: sacrificed"
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    let (_, attackers) = attackers_in_next_declaration(&mut e);
    assert!(
        attackers.contains(&bears),
        "without the Peacekeeper the Bears may attack: {attackers:?}"
    );
}
