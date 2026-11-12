//! A split card is both of its halves in every zone but the stack (CR
//! 709.4): two names, the mana costs combined, and so both colours and the
//! sum for a mana value (CR 709.4b), each half's card types (CR 709.4c). On
//! the stack it is the half being cast and nothing of the other (CR 709.3b).
//!
//! The engine gave such a card its left half everywhere it was not a spell,
//! so Fire // Ice in a hand, a library or a graveyard was a red two: Force of
//! Will could not exile it as a blue card, and a mana value check counted 2
//! where the card has 4 (Carbon b6, attempts 58, 62 and 71, where the seat
//! was shown "Fire" for the card in the graveyard after Ice resolved).

use super::testkit::{
    Duel, RegistryLookup, answer_one, card_index, in_graveyard, in_hand, keep_mulligans, on_stack,
    pass_until, reach_main_phase, stack_is_empty, tap_mana_where,
};
use super::*;
use crate::choice::CastModeKind;
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::types::TypeSet;

/// `{1}{R}` Fire and `{1}{U}` Ice.
fn fire_ice() -> CardIndex {
    card_index("ae92942b-919c-4ea9-b693-85fcef765d5a")
}
fn force_of_will() -> CardIndex {
    card_index("956381ba-6d37-4a8a-846c-bad79222dbee")
}
fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn red_and_blue() -> ColorSet {
    ColorSet::from_slice(&[Color::Red, Color::Blue])
}

/// The name, mana value and colours an object has now.
fn reading(engine: &Engine<RegistryLookup>, id: ObjectId) -> (String, u32, ColorSet) {
    let object = engine.state().object(id).expect("the object");
    let c = object.characteristics();
    (
        engine.state().names.get(c.name).to_string(),
        c.mana_value(),
        c.colors,
    )
}

/// Seat 0's main phase with Fire // Ice in hand, `lands` on its side and a
/// Llanowar Elves on seat 1's.
fn table(lands: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(709, mountain())
        .battlefield(0, lands)
        .hand(0, &[fire_ice()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, ME);
    engine
}

/// Casts the card in seat 0's hand as `way`, aimed at seat 1 or its Elves,
/// and lets it resolve.
fn cast_and_resolve(engine: &mut Engine<RegistryLookup>, way: CastModeKind) -> ObjectId {
    let card = in_hand(engine, ME, fire_ice()).expect("Fire // Ice in hand");
    engine
        .apply(ME, PlayerAction::CastSpell { card })
        .expect("the card is offered");
    for _ in 0..16 {
        let action = match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => PlayerAction::ChooseMode(
                options
                    .iter()
                    .position(|o| o.kind == way)
                    .unwrap_or_else(|| panic!("{way:?} is offered: {options:?}")),
            ),
            Pending::ChooseTargets { player_options, .. } if way == CastModeKind::Normal => {
                assert!(player_options.contains(&THEM), "Fire may aim at a player");
                PlayerAction::ChooseTargets {
                    objects: Vec::new(),
                    players: vec![THEM],
                }
            }
            Pending::Priority { .. } => break,
            _ => answer_one(engine).expect("a cast's question").1,
        };
        engine.apply(ME, action).expect("the cast's answer");
    }
    let (name, mana_value, colors) = reading(engine, card);
    let half = if way == CastModeKind::Normal {
        ("Fire", Color::Red)
    } else {
        ("Ice", Color::Blue)
    };
    assert!(on_stack(engine, fire_ice()).is_some(), "the half is cast");
    assert_eq!(
        (name.as_str(), mana_value, colors),
        (half.0, 2, ColorSet::of(half.1)),
        "on the stack only the half cast exists (CR 709.3b)"
    );
    pass_until(engine, stack_is_empty);
    in_graveyard(engine, ME, fire_ice()).expect("the spell resolved into the graveyard")
}

/// In hand, Fire // Ice is a blue and red instant of mana value 4.
#[test]
fn a_split_card_in_hand_is_both_halves() {
    let engine = table(&[]);
    let card = in_hand(&engine, ME, fire_ice()).expect("Fire // Ice in hand");
    let c = engine
        .state()
        .object(card)
        .expect("the card")
        .characteristics();
    assert_eq!(c.mana_value(), 4, "{{1}}{{R}} and {{1}}{{U}} (CR 709.4b)");
    assert_eq!(c.colors, red_and_blue(), "both colours (CR 709.4b)");
    assert_eq!(c.mana_cost, "{2}{U}{R}".parse().expect("a cost"));
    assert!(
        c.types.contains(TypeSet::INSTANT),
        "each half's type (CR 709.4c)"
    );
}

/// Fire is still cast for `{1}{R}` with only a Mountain and a Plains open,
/// is Fire alone on the stack, and is the whole card again in the graveyard.
#[test]
fn fire_is_cast_for_its_own_cost_and_rests_as_the_whole_card() {
    let mut engine = table(&[mountain(), plains()]);
    tap_mana_where(&mut engine, ME, |_| true);
    let card = cast_and_resolve(&mut engine, CastModeKind::Normal);
    assert_eq!(engine.state().players[1].life, 18, "Fire dealt its 2");
    let (_, mana_value, colors) = reading(&engine, card);
    assert_eq!((mana_value, colors), (4, red_and_blue()));
}

/// Ice the same way, for `{1}{U}`: the card it leaves in the graveyard is
/// not Fire.
#[test]
fn ice_is_cast_for_its_own_cost_and_rests_as_the_whole_card() {
    let mut engine = table(&[island(), plains()]);
    tap_mana_where(&mut engine, ME, |_| true);
    let card = cast_and_resolve(&mut engine, CastModeKind::Face(1));
    let (_, mana_value, colors) = reading(&engine, card);
    assert_eq!((mana_value, colors), (4, red_and_blue()));
}

/// Force of Will may be cast by exiling a blue card from hand (its
/// alternative cost), and Fire // Ice is one (CR 709.4b): seat 1 answers
/// an Elf with it and nothing else in hand.
#[test]
fn force_of_will_may_exile_fire_ice_as_a_blue_card() {
    let mut engine = Duel::new(709, mountain())
        .battlefield(0, &[forest()])
        .hand(0, &[llanowar_elves()])
        .hand(1, &[force_of_will(), fire_ice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, ME);
    tap_mana_where(&mut engine, ME, |_| true);
    let elves = in_hand(&engine, ME, llanowar_elves()).expect("the Elves in hand");
    engine
        .apply(ME, PlayerAction::CastSpell { card: elves })
        .expect("a Forest pays the Elves");
    engine
        .apply(ME, PlayerAction::PassPriority)
        .expect("seat 0 passes");
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("seat 1 holds priority: {:?}", engine.pending());
    };
    assert_eq!(player, THEM);
    let force = in_hand(&engine, THEM, force_of_will()).expect("Force of Will in hand");
    assert!(
        legal.castable.contains(&force),
        "Fire // Ice is a blue card to exile: {:?}",
        legal.castable
    );
}

/// A build from before the split boundary read every split card off the
/// stack by its left half, and its hashes differ from this build's by those
/// cards alone: `snapshot_hash_reading_split_cards_by_left_half` is that
/// build's hash, which a replay audit names a record by. A table without a
/// split card hashes the same either way.
#[test]
fn the_hash_before_the_split_boundary_differs_only_where_a_split_card_is() {
    let with = table(&[mountain()]);
    assert_ne!(
        with.snapshot_hash(),
        with.snapshot_hash_reading_split_cards_by_left_half(),
        "Fire // Ice in hand is read otherwise"
    );
    let mut without = Duel::new(709, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut without);
    reach_main_phase(&mut without, ME);
    assert_eq!(
        without.snapshot_hash(),
        without.snapshot_hash_reading_split_cards_by_left_half()
    );
}

/// The engine as it was before the split boundary
/// (`object::SPLIT_BY_LEFT_HALF`) opens the same game with the hash
/// `snapshot_hash_reading_split_cards_by_left_half` gives for it now, which
/// is what lets a replay audit name a record written before the boundary;
/// and that engine reads Fire // Ice in hand as Fire.
#[test]
fn the_engine_as_it_was_opens_with_the_hash_before_the_boundary() {
    let now = table(&[mountain()]);
    crate::object::SPLIT_BY_LEFT_HALF.with(|s| s.set(true));
    let before = table(&[mountain()]);
    crate::object::SPLIT_BY_LEFT_HALF.with(|s| s.set(false));
    assert_eq!(
        before.snapshot_hash(),
        now.snapshot_hash_reading_split_cards_by_left_half()
    );
    let card = in_hand(&before, ME, fire_ice()).expect("Fire // Ice in hand");
    assert_eq!(
        reading(&before, card),
        ("Fire".to_string(), 2, ColorSet::of(Color::Red))
    );
}
