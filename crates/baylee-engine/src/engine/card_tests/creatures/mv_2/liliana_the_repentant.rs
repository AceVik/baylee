//! `cards/creatures/mv_2/liliana_the_repentant.rs`, played.
//!
//! The mill trigger and the first reanimation live in
//! `creatures/mv_1/llanowar_elves.rs`, and the exhaust bookkeeping in
//! `activation_cost_bounds.rs`; this file pins the two limits of the
//! reanimation's wording: when it may be activated and what it may point at.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Whether printed ability 1 (the exhaust reanimation) of `source` is on the
/// offer right now.
fn exhaust_offered(e: &Engine<RegistryLookup>, source: ObjectId) -> bool {
    matches!(e.pending(), Pending::Priority { legal, .. } if legal.abilities.contains(&(source, 1)))
}

/// Floats plenty of black (and one green, for the Elves spell) and
/// republishes the offer.
fn float_for_exhaust(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let pool = &mut e.dev_state_mut(seat).unwrap().players[0].mana_pool;
    pool.add(ManaColor::Black, 20);
    pool.add(ManaColor::Green, 1);
    e.refresh_offer();
}

/// "Activate only as a sorcery": the same Liliana, graveyard and floating
/// mana are on the offer in p0's main phase with an empty stack, and are not
/// with a creature spell on the stack or on the opponent's turn. The exhaust
/// is still unspent at the last moment, so it is the timing and not the
/// "once only" that withholds it.
#[test]
fn liliana_exhaust_is_offered_only_as_a_sorcery() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(72, llanowar_elves())
        .battlefield(0, &[liliana_the_repentant()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    seed_graveyard(&mut e, p0, 1);
    assert!(
        walk_to_own_main(&mut e, p0),
        "p0 reaches its own main phase"
    );
    let liliana = on_battlefield(&e, p0, liliana_the_repentant()).expect("she is out");
    assert!(
        in_graveyard(&e, p0, llanowar_elves()).is_some(),
        "a creature card waits in the graveyard"
    );
    float_for_exhaust(&mut e, p0);
    assert!(
        exhaust_offered(&e, liliana),
        "control: main phase, empty stack, {{5}}{{B}} floating, a creature card to return"
    );

    cast_with_floating(&mut e, p0, llanowar_elves());
    assert!(!stack_is_empty(&e), "the Elves spell is waiting to resolve");
    assert!(
        e.state().players[0].mana_pool.available(ManaColor::Black) >= 6,
        "the {{5}}{{B}} is still floating, so mana is not what withholds the ability"
    );
    assert!(
        !exhaust_offered(&e, liliana),
        "\"only as a sorcery\": not while a spell is on the stack"
    );
    let hash = e.state().snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: liliana,
                ability_index: 1
            }
        )
        .is_err()
    );
    assert_eq!(
        e.state().snapshot_hash(),
        hash,
        "the refusal changed nothing"
    );
    pass_until(&mut e, stack_is_empty);

    pass_until(&mut e, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_for_exhaust(&mut e, p0);
    assert!(
        in_graveyard(&e, p0, llanowar_elves()).is_some(),
        "the creature card is still there on the opponent's turn"
    );
    assert!(
        !exhaust_offered(&e, liliana),
        "\"only as a sorcery\": not on the opponent's turn, though nothing has used the exhaust"
    );
    let hash = e.state().snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: liliana,
                ability_index: 1
            }
        )
        .is_err()
    );
    assert_eq!(
        e.state().snapshot_hash(),
        hash,
        "the refusal changed nothing"
    );
}

/// "Return target creature or planeswalker card from **your** graveyard": a
/// creature and a planeswalker card of p0's are on the menu; an instant of
/// p0's and a creature card in the opponent's graveyard are not.
#[test]
fn liliana_exhaust_targets_only_creature_or_planeswalker_cards_in_your_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[liliana_the_repentant()])
        .hand(
            0,
            &[grizzly_bears(), giant_growth(), karn_the_great_creator()],
        )
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    assert!(
        walk_to_own_main(&mut e, p0),
        "p0 reaches its own main phase"
    );
    let liliana = on_battlefield(&e, p0, liliana_the_repentant()).expect("she is out");
    let bears = in_hand(&e, p0, grizzly_bears()).unwrap();
    let growth = in_hand(&e, p0, giant_growth()).unwrap();
    let karn = in_hand(&e, p0, karn_the_great_creator()).unwrap();
    let their_elves = in_hand(&e, p1, llanowar_elves()).unwrap();
    {
        let state = e.dev_state_mut(p0).unwrap();
        for (card, owner) in [(bears, p0), (growth, p0), (karn, p0), (their_elves, p1)] {
            state
                .move_object(
                    card,
                    ZoneLocation::Graveyard(owner),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Setup,
                )
                .unwrap();
        }
    }
    float_for_exhaust(&mut e, p0);
    assert!(
        exhaust_offered(&e, liliana),
        "a creature and a planeswalker card of p0's are targets, so the ability is offered"
    );
    activate(&mut e, p0, liliana_the_repentant(), 1);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the reanimation asks which card: {:?}", e.pending())
    };
    assert!(
        options.contains(&bears) && options.contains(&karn),
        "a creature card and a planeswalker card in your graveyard are both on the menu: {options:?}"
    );
    assert!(
        !options.contains(&growth),
        "an instant card is neither a creature nor a planeswalker card: {options:?}"
    );
    assert!(
        !options.contains(&their_elves),
        "\"from your graveyard\": the opponent's creature card is not: {options:?}"
    );
}
