//! Fight (CR 701.14), and the second instance of the word "target" that every
//! fight between two chosen creatures needs.
//!
//! The cards are examples here and not the subject — each of them has its own
//! scenario behind its own door in `card_tests`. What is tested in this file
//! is the part that can go wrong without any card being wrong:
//!
//! - CR 701.14b: a fight with one side gone deals damage to **neither** side,
//!   in both directions.
//! - CR 608.2b "for every instance of the word 'target'": each instance is
//!   re-checked against its own requirement, a spell with one legal instance
//!   still resolves, and a first creature that became illegal does not make
//!   the second one the target of the effects that read the first.
//! - A pump and a fight in one resolution fight at the pumped power, which is
//!   the projection being refreshed between effects rather than between
//!   engine steps.
//! - CR 601.2c: a spell is castable only when every instance can be filled.
//! - The second instance is part of the position: two games differing only in
//!   what the fight's second creature is hash apart.

use super::testkit::{
    Duel, RegistryLookup, card_index, cast_with_floating, in_graveyard, keep_mulligans,
    on_battlefield, pass_until, pt, stack_is_empty, tap_all_mana, walk_to_own_main,
};
use super::*;
use crate::choice::CastModeKind;
use baylee_core::ids::{CardIndex, ObjectId};

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// A 1/1.
fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// A 2/2 with haste.
fn wild_colos() -> CardIndex {
    card_index("cb6b8ce3-9f9d-418c-94b0-c4469a254938")
}

/// A 4/4 Beast with trample.
fn fangren_hunter() -> CardIndex {
    card_index("c5dc5546-e9e5-4b5b-b812-5716d4bdee0e")
}

/// A 1/1 with flying and deathtouch.
fn baleful_strix() -> CardIndex {
    card_index("37688720-03de-4eca-a82d-a0afe8d58adc")
}

/// "Target creature you control fights target creature you don't control."
fn khalni_ambush() -> CardIndex {
    card_index("37a55560-6e32-4f54-b9a8-fd157aea6eb5")
}

/// "Target creature you control gets +2/+2 until end of turn. It fights up to
/// one target creature you don't control."
fn bridgeworks_battle() -> CardIndex {
    card_index("9d581188-ce80-494e-bd38-f411e1f4efb5")
}

/// The damage marked on a permanent.
#[track_caller]
fn damage(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .expect("the permanent is still an object")
        .damage
}

/// Casts the front face of a modal double-faced card off floating mana.
#[track_caller]
fn cast_front(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) {
    cast_with_floating(engine, seat, card);
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let slot = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Face(0)))
            .expect("the front face is one of the ways to play this card");
        engine
            .apply(seat, PlayerAction::ChooseMode(slot))
            .expect("the front face is a legal choice");
    }
}

/// Answers the target question in front of `seat` with `objects`, and hands
/// back the menu it offered with its bounds.
#[track_caller]
fn name(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    objects: &[ObjectId],
) -> (Vec<ObjectId>, u8, u8) {
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, seat, "the caster names the targets");
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: objects.to_vec(),
                players: vec![],
            },
        )
        .expect("the named targets were on the menu");
    (options, min, max)
}

/// A duel on p0's main phase with three Forests, `mine` beside them and
/// `theirs` across the table, and `spell` in hand.
fn table(
    seed: u64,
    spell: CardIndex,
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut board = vec![forest(), forest(), forest()];
    board.extend_from_slice(mine);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &board)
        .battlefield(1, theirs)
        .hand(0, &[spell])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, PlayerId::new(0)),
        "p0 reaches its own main"
    );
    engine
}

/// Destroys permanents on the spot, without handing priority over — the
/// window between a spell being cast and resolving, which is what an opponent
/// answering it would use.
#[track_caller]
fn bury(engine: &mut Engine<RegistryLookup>, ids: &[ObjectId]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for id in ids {
        crate::sba::destroy(state, *id);
    }
}

/// CR 701.14b, the half that is easy to get backwards: the creature that is
/// still there does not hit anything on its own.
///
/// Khalni Ambush names a 1/1 of mine and a 2/2 of theirs; the 2/2 dies before
/// the spell resolves. The spell still resolves — its first target is legal,
/// so CR 608.2b does not remove it — and my 1/1 has no damage marked, which
/// is what "neither of them fights or deals damage" says. A resolver that let
/// the surviving side deal its half would have left the Elves untouched too,
/// so the reverse direction is the next test.
#[test]
fn a_fight_whose_second_creature_is_gone_deals_no_damage_at_all() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(7010, khalni_ambush(), &[llanowar_elves()], &[wild_colos()]);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");

    tap_all_mana(&mut engine, p0);
    cast_front(&mut engine, p0, khalni_ambush());
    name(&mut engine, p0, &[elves]);
    name(&mut engine, p0, &[colos]);
    bury(&mut engine, &[colos]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, khalni_ambush()).is_some(),
        "the spell resolved or was removed; either way it left the stack"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "a 1/1 that fought a 2/2 would be dead"
    );
    assert_eq!(damage(&engine, elves), 0, "and it was dealt nothing at all");
}

/// CR 701.14b the other way round: the first creature is gone, and the
/// second is dealt nothing — including by nobody else standing in the gone
/// one's place.
///
/// That second clause is why the two instances are two lists. Bridgeworks
/// Battle pumps its **first** target and then fights; had the second target
/// been appended to the first list, CR 608.2b's narrowing would have dropped
/// my creature and left theirs at index zero, and the opponent's 2/2 would
/// have been given +2/+2 by my own spell.
#[test]
fn a_first_target_that_is_gone_is_not_replaced_by_the_second() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        7011,
        bridgeworks_battle(),
        &[fangren_hunter()],
        &[wild_colos()],
    );
    let hunter = on_battlefield(&engine, p0, fangren_hunter()).expect("my Hunter is out");
    let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");

    tap_all_mana(&mut engine, p0);
    cast_front(&mut engine, p0, bridgeworks_battle());
    name(&mut engine, p0, &[hunter]);
    name(&mut engine, p0, &[colos]);
    bury(&mut engine, &[hunter]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, colos),
        (2, 2),
        "their creature is not given the pump that named mine"
    );
    assert_eq!(damage(&engine, colos), 0, "and the fight dealt it nothing");
    assert!(on_battlefield(&engine, p1, wild_colos()).is_some());
}

/// "Gets +2/+2 until end of turn. It fights …" — the fight is at the pumped
/// power, because a continuous effect applies the moment it exists (CR 613)
/// and not at the next engine step.
///
/// A 1/1 pumped to 3/3 fights a 2/2: the 2/2 dies and the 3/3 survives with
/// two damage. Read at the stale 1/1 it would deal one and die to two, which
/// is the board this test was written against — the projection used to be
/// refreshed only between engine steps.
#[test]
fn a_pump_and_a_fight_in_one_resolution_fight_at_the_pumped_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        7012,
        bridgeworks_battle(),
        &[llanowar_elves()],
        &[wild_colos()],
    );
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");

    tap_all_mana(&mut engine, p0);
    cast_front(&mut engine, p0, bridgeworks_battle());
    name(&mut engine, p0, &[elves]);
    name(&mut engine, p0, &[colos]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, wild_colos()).is_some(),
        "three damage from the pumped Elves kill the 2/2"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and a 3/3 survives the 2/2's two"
    );
    assert_eq!(damage(&engine, elves), 2);
    assert_eq!(pt(&engine, elves), (3, 3));
}

/// Deathtouch is a property of the source (CR 702.2b), and in a fight each
/// creature is the source of its own damage (CR 701.14a) — so a 1/1 with
/// deathtouch trades with a 4/4.
#[test]
fn a_deathtouch_creature_kills_what_it_fights() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        7013,
        khalni_ambush(),
        &[baleful_strix()],
        &[fangren_hunter()],
    );
    let strix = on_battlefield(&engine, p0, baleful_strix()).expect("my Strix is out");
    let hunter = on_battlefield(&engine, p1, fangren_hunter()).expect("their Hunter is out");

    tap_all_mana(&mut engine, p0);
    cast_front(&mut engine, p0, khalni_ambush());
    name(&mut engine, p0, &[strix]);
    name(&mut engine, p0, &[hunter]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, fangren_hunter()).is_some(),
        "one damage from a deathtouch source is lethal (CR 704.5h)"
    );
    assert!(
        in_graveyard(&engine, p0, baleful_strix()).is_some(),
        "and the Strix took four"
    );
}

/// CR 601.2c chooses a target for **every** instance, so a spell whose
/// second instance cannot be filled cannot be cast — however many creatures
/// the caster has for the first.
///
/// Held against the same board with one creature added across the table,
/// which is what makes it the second requirement doing the refusing and not
/// the mana or the first.
#[test]
fn a_spell_is_castable_only_when_every_instance_of_target_can_be_filled() {
    let p0 = PlayerId::new(0);
    let offered = |theirs: &[CardIndex]| {
        let mut engine = table(7014, khalni_ambush(), &[fangren_hunter()], theirs);
        // The offer is priced against what is floating, so the mana is made
        // first and the only thing left to decide the answer is the targets.
        tap_all_mana(&mut engine, p0);
        let ambush = engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .iter()
            .copied()
            .find(|id| {
                engine
                    .state()
                    .object(*id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index == khalni_ambush())
            })
            .expect("the Ambush is in hand");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        legal.castable.contains(&ambush)
    };
    assert!(
        !offered(&[]),
        "no creature on the other side: the second instance has nothing to name"
    );
    assert!(
        offered(&[wild_colos()]),
        "one creature there, and it is a spell"
    );
}

/// "Up to one" is an answer of none, and the rest of the spell happens
/// (CR 608.2b is about targets that were chosen, and none was).
#[test]
fn an_up_to_one_fight_declined_still_pumps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        7015,
        bridgeworks_battle(),
        &[llanowar_elves()],
        &[wild_colos()],
    );
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");

    tap_all_mana(&mut engine, p0);
    cast_front(&mut engine, p0, bridgeworks_battle());
    name(&mut engine, p0, &[elves]);
    let (_, min, max) = name(&mut engine, p0, &[]);
    assert_eq!((min, max), (0, 1), "the printing says \"up to one\"");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elves),
        (3, 3),
        "the pump is not the fight's to cancel"
    );
    assert_eq!(damage(&engine, elves), 0);
    assert_eq!(damage(&engine, colos), 0);
}

/// The second instance is part of the position. Two games that are the same
/// in every other respect, with the fight's second creature named
/// differently, are different games — and a hash that did not read the list
/// would have called them one, which is what loop detection and replay
/// comparison stand on.
#[test]
fn the_second_instance_is_part_of_the_hashed_position() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let hashed = |pick_colos: bool| {
        let mut engine = table(
            7016,
            khalni_ambush(),
            &[fangren_hunter()],
            &[wild_colos(), llanowar_elves()],
        );
        let hunter = on_battlefield(&engine, p0, fangren_hunter()).expect("my Hunter is out");
        let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");
        let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
        tap_all_mana(&mut engine, p0);
        cast_front(&mut engine, p0, khalni_ambush());
        name(&mut engine, p0, &[hunter]);
        name(&mut engine, p0, &[if pick_colos { colos } else { elves }]);
        engine.snapshot_hash()
    };
    assert_eq!(hashed(true), hashed(true), "the same game hashes the same");
    assert_ne!(
        hashed(true),
        hashed(false),
        "a different second target is a different position"
    );
}
