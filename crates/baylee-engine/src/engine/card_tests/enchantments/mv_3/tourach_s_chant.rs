//! `cards/enchantments/mv_3/tourach_s_chant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chant_card() -> CardIndex {
    card_index("bdf4e225-23b5-47d9-9936-ddcec8aa1ca4")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn life(e: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    e.state().players[seat.get() as usize].life
}

fn m1m1(e: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    e.state()
        .object(id)
        .expect("on the battlefield")
        .counters
        .get(CounterKind::M1M1)
}

fn choose_cards_pending(e: &Engine<RegistryLookup>) -> bool {
    matches!(e.pending(), Pending::ChooseCards { .. })
}

/// p0: three Swamps, a Grizzly Bears, Tourach's Chant and a Forest in hand.
/// The Chant is cast and resolved in the first main phase.
fn chant_cast() -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, island())
        .hand(0, &[chant_card(), forest(), swamp()])
        .battlefield(0, &[swamp(), swamp(), swamp(), bears_card()])
        .battlefield(1, &[bears_card()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast_from_hand(&mut e, P0, chant_card());
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, chant_card()).is_some());
    e
}

fn pay_with_every_land(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    e.apply(seat, PlayerAction::YesNo(true))
        .expect("the price can be answered");
    tap_all_mana(e, seat);
    e.apply(seat, PlayerAction::PassPriority)
        .expect("closing the payment window");
    pass_until(e, stack_is_empty);
}

/// Tourach's Chant (bdf4e225-23b5-47d9-9936-ddcec8aa1ca4) — "Whenever a player
/// puts a Forest onto the battlefield, this enchantment deals 3 damage to that
/// player unless the player puts a -1/-1 counter on a creature they control."
/// The player puts the counter and takes nothing.
#[test]
fn tourach_s_chant_a_forest_costs_a_counter_instead_of_the_damage() {
    let mut e = chant_cast();
    let bears = on_battlefield(&e, P0, bears_card()).expect("my Bears");
    play_land(&mut e, P0, forest());
    pass_until(&mut e, choose_cards_pending);
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0, "the Forest's player decides");
    assert_eq!(options, vec![bears], "only a creature they control");
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .expect("the Bears take the counter");
    pass_until(&mut e, stack_is_empty);

    assert_eq!(m1m1(&e, bears), 1, "a -1/-1 counter");
    assert_eq!(pt(&e, bears), (1, 1));
    assert_eq!(life(&e, P0), 20, "no damage");
}

/// Tourach's Chant — declining the counter takes the 3 damage.
#[test]
fn tourach_s_chant_a_forest_without_the_counter_deals_three() {
    let mut e = chant_cast();
    let bears = on_battlefield(&e, P0, bears_card()).expect("my Bears");
    play_land(&mut e, P0, forest());
    pass_until(&mut e, choose_cards_pending);
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("choosing none is allowed");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life(&e, P0), 17, "3 damage");
    assert_eq!(m1m1(&e, bears), 0);
}

/// Tourach's Chant — with no creature to put a counter on, there is no
/// question: the damage is dealt.
#[test]
fn tourach_s_chant_without_a_creature_just_deals_three() {
    let mut e = Duel::new(SEED, island())
        .hand(0, &[chant_card(), forest()])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast_from_hand(&mut e, P0, chant_card());
    pass_until(&mut e, stack_is_empty);
    play_land(&mut e, P0, forest());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life(&e, P0), 17);
}

/// Tourach's Chant — "a player": the opponent's Forest is punished too, with
/// the opponent's own creatures on the menu.
#[test]
fn tourach_s_chant_punishes_the_opponents_forest_too() {
    let mut e2 = Duel::new(SEED, island())
        .hand(0, &[chant_card()])
        .hand(1, &[forest()])
        .battlefield(0, &[swamp(), swamp(), swamp(), bears_card()])
        .battlefield(1, &[bears_card()])
        .start();
    keep_mulligans(&mut e2);
    reach_main_phase(&mut e2, P0);
    cast_from_hand(&mut e2, P0, chant_card());
    pass_until(&mut e2, stack_is_empty);
    let mine = on_battlefield(&e2, P0, bears_card()).expect("my Bears");
    let theirs = on_battlefield(&e2, P1, bears_card()).expect("their Bears");

    // P1's turn: a Forest from its hand. The Chant's upkeep is p0's only.
    reach_their_main_phase(&mut e2, P1);
    play_land(&mut e2, P1, forest());
    pass_until(&mut e2, choose_cards_pending);
    let Pending::ChooseCards {
        player, options, ..
    } = e2.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P1, "the player who put the Forest out is asked");
    assert!(options.contains(&theirs));
    assert!(!options.contains(&mine), "not a creature p0 controls");
    e2.apply(P1, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("decline");
    pass_until(&mut e2, stack_is_empty);
    assert_eq!(life(&e2, P1), 17, "p1 takes the 3");
    assert_eq!(life(&e2, P0), 20);
}

/// Tourach's Chant — only a Forest: a Swamp from the same hand trips nothing.
#[test]
fn tourach_s_chant_ignores_other_lands() {
    let mut e = chant_cast();
    play_land(&mut e, P0, swamp());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life(&e, P0), 20);
}

/// Tourach's Chant — "At the beginning of your upkeep, sacrifice this
/// enchantment unless you pay {B}." Paid: it stays.
#[test]
fn tourach_s_chant_upkeep_paid_keeps_it() {
    let mut e = chant_cast();
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = e.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0);
    assert_eq!(e.state().turn.active, P0);
    assert_eq!(
        prompt,
        YesNoPrompt::PayMana {
            cost: baylee_core::mana!("{B}")
        }
    );
    pay_with_every_land(&mut e, P0);
    assert!(on_battlefield(&e, P0, chant_card()).is_some());
}

/// Tourach's Chant — unpaid: it is sacrificed.
#[test]
fn tourach_s_chant_upkeep_declined_sacrifices_it() {
    let mut e = chant_cast();
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    e.apply(P0, PlayerAction::YesNo(false)).expect("decline");
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, chant_card()).is_none());
    assert!(in_graveyard(&e, P0, chant_card()).is_some());
}
