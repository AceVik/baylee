//! `cards/enchantments/mv_2/thelon_s_curse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn curse_card() -> CardIndex {
    card_index("f6880f71-a05a-4752-ba33-ffbc450606bf")
}
/// {U} 1/1, no rules text: a blue creature.
fn merfolk_card() -> CardIndex {
    card_index("218d9277-c179-4de3-9c7f-79b5a6d4fa38")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn untap_question(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::Untap,
            ..
        }
    )
}

/// Passes priority until somebody is asked which tapped creatures to untap,
/// or until p0 reaches a later first main phase with nothing asked.
/// `pass_until` answers the untap question by itself, so the stop is ours.
#[track_caller]
fn walk_to_the_choice(engine: &mut Engine<RegistryLookup>) -> Option<(PlayerId, Vec<ObjectId>)> {
    let began = engine.state().turn.number;
    pass_until(engine, |e| {
        untap_question(e)
            || (e.state().turn.number > began
                && e.state().turn.active == P0
                && matches!(e.state().turn.phase, Phase::FirstMain))
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        return None;
    };
    Some((player, options))
}

fn is_tapped(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .expect("object exists")
        .status
        .contains(Status::TAPPED)
}

/// Answers the choice, and returns the price asked next when something was
/// chosen.
#[track_caller]
fn choose(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    chosen: &[ObjectId],
) -> Option<baylee_core::mana::ManaCost> {
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: chosen.to_vec(),
            },
        )
        .expect("the choice came out of the question");
    if chosen.is_empty() {
        return None;
    }
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "choosing creatures asks for the price next, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat, "the price is asked of the one who chose");
    Some(cost)
}

/// "Yes", then every land taps, and the window closes.
#[track_caller]
fn pay_with_every_land(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    engine
        .apply(seat, PlayerAction::YesNo(true))
        .expect("the price can be answered");
    tap_all_mana(engine, seat);
    engine
        .apply(seat, PlayerAction::PassPriority)
        .expect("closing the payment window");
    pass_until(engine, stack_is_empty);
}

/// p0 holds the Curse (when `with_curse`); p1 holds two blue creatures, a
/// green one and two Islands, and its creatures are tapped during p0's main
/// phase, so that p1's untap step is a real one.
fn game(with_curse: bool) -> (Engine<RegistryLookup>, Vec<ObjectId>, ObjectId) {
    let mine: Vec<CardIndex> = if with_curse {
        vec![curse_card()]
    } else {
        vec![]
    };
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(
            1,
            &[
                merfolk_card(),
                merfolk_card(),
                bears_card(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let blues = all_on_battlefield(&e, P1, merfolk_card());
    assert_eq!(blues.len(), 2);
    let bears = on_battlefield(&e, P1, bears_card()).expect("their Bears");
    let state = e.dev_state_mut(P0).expect("the harness may set boards up");
    for id in blues.iter().chain([&bears]) {
        state.set_tapped(*id, true);
    }
    e.refresh_offer();
    (e, blues, bears)
}

/// Thelon's Curse (f6880f71-a05a-4752-ba33-ffbc450606bf) — "Blue creatures
/// don't untap during their controllers' untap steps." Blue ones stay tapped
/// through the untap step, a green one does not, and without the Curse all
/// of them untap.
#[test]
fn thelon_s_curse_keeps_blue_creatures_tapped_through_the_untap_step() {
    let (mut e, blues, bears) = game(true);
    let (asked, options) = walk_to_the_choice(&mut e).expect("p1 has tapped blue creatures");
    assert_eq!(asked, P1, "the upkeep of p1 asks p1");
    assert_eq!(e.state().turn.active, P1);
    assert_eq!(e.state().turn.step, Step::Upkeep);
    for b in &blues {
        assert!(is_tapped(&e, *b), "blue: did not untap");
        assert!(options.contains(b), "and it is on the menu");
    }
    assert!(!is_tapped(&e, bears), "green: untapped as usual");
    assert!(!options.contains(&bears), "only blue creatures are offered");

    // Control: no Curse, no question, everything untaps.
    let (mut bare, blues, bears) = game(false);
    assert!(walk_to_the_choice(&mut bare).is_none(), "nothing is asked");
    for id in blues.iter().chain([&bears]) {
        assert!(
            !is_tapped(&bare, *id),
            "without the Curse everything untaps"
        );
    }
}

/// Thelon's Curse — "At the beginning of each player's upkeep, that player may
/// choose any number of tapped blue creatures they control and pay {U} for
/// each creature chosen this way. If the player does, untap those creatures."
#[test]
fn thelon_s_curse_untaps_the_chosen_creatures_for_u_each() {
    let (mut e, blues, _bears) = game(true);
    walk_to_the_choice(&mut e).expect("the question");
    let cost = choose(&mut e, P1, &blues).expect("a price follows");
    assert_eq!(
        cost,
        baylee_core::mana::ManaCost::parse("{U}{U}"),
        "{{U}} each"
    );
    pay_with_every_land(&mut e, P1);
    for b in &blues {
        assert!(!is_tapped(&e, *b), "paid for: untapped");
    }
}

/// Thelon's Curse — "any number": choosing one pays {U} and leaves the other
/// tapped.
#[test]
fn thelon_s_curse_pays_only_for_the_creatures_chosen() {
    let (mut e, blues, _bears) = game(true);
    walk_to_the_choice(&mut e).expect("the question");
    let cost = choose(&mut e, P1, &blues[..1]).expect("a price follows");
    assert_eq!(cost, baylee_core::mana::ManaCost::parse("{U}"));
    pay_with_every_land(&mut e, P1);
    assert!(!is_tapped(&e, blues[0]), "the chosen one untapped");
    assert!(is_tapped(&e, blues[1]), "the other stays down");
}

/// Thelon's Curse — "If the player does": declining the price leaves the
/// chosen creature tapped.
#[test]
fn thelon_s_curse_declined_payment_leaves_them_tapped() {
    let (mut e, blues, _bears) = game(true);
    walk_to_the_choice(&mut e).expect("the question");
    choose(&mut e, P1, &blues[..1]).expect("a price follows");
    e.apply(P1, PlayerAction::YesNo(false)).expect("decline");
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, blues[0]), "unpaid: still tapped");
}

/// Thelon's Curse — "each player's upkeep": p0's own tapped blue creatures
/// are offered at p0's upkeep too, to p0.
#[test]
fn thelon_s_curse_also_asks_its_own_controller_at_their_upkeep() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[curse_card(), merfolk_card(), island()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let mine = on_battlefield(&e, P0, merfolk_card()).expect("my blue creature");
    e.dev_state_mut(P0)
        .expect("the harness may set boards up")
        .set_tapped(mine, true);
    e.refresh_offer();

    let (asked, options) = walk_to_the_choice(&mut e).expect("p0 is asked at its own upkeep");
    assert_eq!(asked, P0);
    assert_eq!(e.state().turn.active, P0);
    assert_eq!(options, vec![mine]);
    assert!(
        is_tapped(&e, mine),
        "the Curse kept it down in the untap step"
    );
}
