//! `cards/enchantments/mv_2/dance_of_many.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dance_card() -> CardIndex {
    card_index("72e1a239-11a7-4dbe-b093-65d395e3ccc9")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}
fn hill_giant_card() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn tokens(e: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = e.state().object(id).expect("on the battlefield");
            !o.is_card() && o.controller == seat
        })
        .collect()
}

/// P0: two Islands, a Grizzly Bears and Dance of Many in hand; P1: a Hill
/// Giant. Dance of Many is cast and its enter trigger aimed at `target`.
fn dance_cast_at(target: CardIndex, target_seat: PlayerId) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[dance_card()])
        .battlefield(0, &[island(), island(), bears_card()])
        .battlefield(1, &[hill_giant_card()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast_from_hand(&mut e, P0, dance_card());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let victim = on_battlefield(&e, target_seat, target).expect("the creature to copy");
    let menu = aim_at(&mut e, P0, victim);
    assert!(
        menu.contains(&victim),
        "a nontoken creature is a legal target"
    );
    pass_until(&mut e, stack_is_empty);
    e
}

/// Pays the question by tapping every land, as a player would, and lets the
/// trigger finish.
fn pay_with_every_land(e: &mut Engine<RegistryLookup>) {
    e.apply(P0, PlayerAction::YesNo(true))
        .expect("the price can be answered");
    tap_all_mana(e, P0);
    e.apply(P0, PlayerAction::PassPriority)
        .expect("closing the payment window");
    pass_until(e, stack_is_empty);
}

fn to_the_upkeep_question(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = e.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0, "\"your upkeep\" asks Dance's controller");
    assert_eq!(
        prompt,
        crate::choice::YesNoPrompt::PayMana {
            cost: baylee_core::mana!("{U}{U}")
        }
    );
}

/// Dance of Many (72e1a239-11a7-4dbe-b093-65d395e3ccc9) — "When this
/// enchantment enters, create a token that's a copy of target nontoken
/// creature."
#[test]
fn dance_of_many_copies_the_target_creature_even_an_opponents() {
    let e = dance_cast_at(hill_giant_card(), P1);
    let made = tokens(&e, P0);
    assert_eq!(
        made.len(),
        1,
        "one token, under the Enchantment's controller"
    );
    assert_eq!(
        pt(&e, made[0]),
        (3, 3),
        "a copy of Hill Giant, not of the Bears"
    );
    assert!(
        on_battlefield(&e, P1, hill_giant_card()).is_some(),
        "the original stays where it was"
    );
}

/// Dance of Many — "When this enchantment leaves the battlefield, exile the
/// token."
#[test]
fn dance_of_many_leaving_exiles_the_token() {
    let mut e = dance_cast_at(bears_card(), P0);
    let made = tokens(&e, P0);
    assert_eq!(made.len(), 1);
    assert_eq!(pt(&e, made[0]), (2, 2));

    let dance = on_battlefield(&e, P0, dance_card()).expect("Dance of Many is out");
    kill(&mut e, dance);
    assert!(
        tokens(&e, P0).is_empty(),
        "the token left the battlefield with the Enchantment"
    );
    assert!(
        on_battlefield(&e, P0, bears_card()).is_some(),
        "the real Bears are untouched"
    );
}

/// Dance of Many — "When the token leaves the battlefield, sacrifice this
/// enchantment."
#[test]
fn dance_of_many_is_sacrificed_when_the_token_leaves() {
    let mut e = dance_cast_at(bears_card(), P0);
    let made = tokens(&e, P0);
    assert_eq!(made.len(), 1);

    kill(&mut e, made[0]);
    assert!(
        on_battlefield(&e, P0, dance_card()).is_none(),
        "the Enchantment is sacrificed"
    );
    assert!(in_graveyard(&e, P0, dance_card()).is_some());
}

/// Dance of Many — "At the beginning of your upkeep, sacrifice this
/// enchantment unless you pay {U}{U}." Paying keeps it and the token.
#[test]
fn dance_of_many_upkeep_paid_keeps_it_and_the_token() {
    let mut e = dance_cast_at(bears_card(), P0);
    to_the_upkeep_question(&mut e);
    pay_with_every_land(&mut e);
    assert!(
        on_battlefield(&e, P0, dance_card()).is_some(),
        "paying {{U}}{{U}} keeps it"
    );
    assert_eq!(tokens(&e, P0).len(), 1, "and so the token");
}

/// Dance of Many — declining the upkeep payment sacrifices it, and with it
/// the token goes.
#[test]
fn dance_of_many_upkeep_declined_sacrifices_it_and_the_token_goes() {
    let mut e = dance_cast_at(bears_card(), P0);
    to_the_upkeep_question(&mut e);
    e.apply(P0, PlayerAction::YesNo(false)).expect("decline");
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, dance_card()).is_none());
    assert!(in_graveyard(&e, P0, dance_card()).is_some());
    assert!(tokens(&e, P0).is_empty(), "the token is exiled with it");
}
