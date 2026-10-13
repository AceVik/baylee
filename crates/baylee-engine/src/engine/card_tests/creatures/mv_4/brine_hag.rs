//! `cards/creatures/mv_4/brine_hag.rs`, played.
//!
//! "When this creature dies, the base power and toughness of each creature that
//! dealt damage to it this turn become 0/2. (This effect lasts indefinitely.)"
//! A 2/2.
//!
//! A creature with 2 damage on it and a toughness of 2 dies (CR 704.5g), so
//! every board below is built so that the creatures the Hag turns into 0/2 are
//! carrying less than 2 damage when it happens: the Hag's 2 damage is split
//! 1 and 1, or lands on a creature whose counter keeps its toughness at 3.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use baylee_core::generated::index;

// oracle_id = "e0165326-f1a6-4fc0-94c1-4b33f24d36f5"
fn brine_hag() -> CardIndex {
    card_index("e0165326-f1a6-4fc0-94c1-4b33f24d36f5")
}

/// Puts a +1/+1 counter on `object`, the way a dev board seeds a counter, and
/// lets the engine look at it again.
#[track_caller]
fn put_a_plus_one_counter_on(engine: &mut Engine<RegistryLookup>, object: ObjectId) {
    engine
        .dev_state_mut(kit::P0)
        .expect("the dev state of a test engine")
        .object_mut(object)
        .expect("the object is on the table")
        .counters
        .set(baylee_cards_dsl::CounterKind::P1P1, 1);
    engine.refresh_offer();
}

/// "The base power and toughness of each creature that dealt damage to it this
/// turn become 0/2": the Hag attacks and two Bears block it. The Hag splits its
/// 2 damage 1 and 1 (so neither Bear is lethally hurt), the Bears deal 4 to a
/// 2/2, and both Bears become 0/2 — each with its 1 damage still marked. A
/// third Bear that dealt it nothing stays 2/2. "Lasts indefinitely": the two
/// Bears are still 0/2 in the next turn, after the damage has healed, and the
/// bystander is still 2/2.
#[test]
fn brine_hag_dying_makes_every_creature_that_dealt_damage_to_it_a_0_2() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[brine_hag()])
        .battlefield(1, &[kit::bears(), kit::bears(), kit::bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let hag = on_battlefield(&engine, kit::P0, brine_hag()).expect("the Hag is out");
    let bears = kit::objects_of(&engine, kit::P1, kit::bears());
    assert_eq!(bears.len(), 3, "three Bears across the table");
    let (first, second, bystander) = (bears[0], bears[1], bears[2]);
    for bear in &bears {
        assert_eq!(
            kit::power_toughness(&engine, *bear),
            (2, 2),
            "before the fight every Bear is a 2/2"
        );
    }

    kit::attack(&mut engine, kit::P0, &[hag], kit::P1);
    kit::block(&mut engine, kit::P1, &[(first, hag), (second, hag)]);
    let mut answers = kit::Answers::new().share(1);
    kit::drive_with(&mut engine, &mut answers, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P0)
    });

    assert!(
        in_graveyard(&engine, kit::P0, brine_hag()).is_some(),
        "two Bears dealt 4 to a 2/2: the Hag died"
    );
    for bear in [first, second] {
        assert!(
            kit::on_the_table(&engine, bear),
            "the Hag dealt 1 to each Bear, which is not lethal to a 2/2 or a 0/2"
        );
        assert_eq!(
            kit::damage_on(&engine, bear),
            1,
            "the Hag's damage was split 1 and 1"
        );
        assert_eq!(
            kit::power_toughness(&engine, bear),
            (0, 2),
            "a Bear that dealt damage to the Hag has base power and toughness 0/2"
        );
    }
    assert_eq!(
        kit::power_toughness(&engine, bystander),
        (2, 2),
        "the Bear that dealt the Hag no damage is unchanged"
    );

    // "This effect lasts indefinitely": into the next turn, damage healed.
    kit::walk_to_next_own_main(&mut engine, kit::P1);
    for bear in [first, second] {
        assert_eq!(
            kit::damage_on(&engine, bear),
            0,
            "damage wore off at cleanup"
        );
        assert_eq!(
            kit::power_toughness(&engine, bear),
            (0, 2),
            "and the 0/2 did not: it lasts indefinitely, not until end of turn"
        );
    }
    assert_eq!(
        kit::power_toughness(&engine, bystander),
        (2, 2),
        "the bystander is still a 2/2 a turn later"
    );
}

/// "The base power and toughness" — base: the effect sets layer 7b and a +1/+1
/// counter still modifies the result afterwards (CR 613.4b, CR 613.4c). The Hag
/// blocks a Bear carrying a +1/+1 counter (a 3/3), takes 3 and dies; the Bear
/// took 2 and, as a base 0/2 with its counter, is a 1/3 with 2 damage on it
/// and lives. The other attacking Bear, which the Hag did not block, dealt it
/// nothing: it stays a 2/2 and its 2 damage reaches the player.
#[test]
fn brine_hag_dying_while_blocking_sets_the_base_and_the_counter_still_applies() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[brine_hag()])
        .battlefield(1, &[kit::bears(), kit::bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let hag = on_battlefield(&engine, kit::P0, brine_hag()).expect("the Hag is out");
    let bears = kit::objects_of(&engine, kit::P1, kit::bears());
    assert_eq!(bears.len(), 2, "two Bears to attack with");
    let (blocked, unblocked) = (bears[0], bears[1]);
    put_a_plus_one_counter_on(&mut engine, blocked);

    kit::attack(&mut engine, kit::P1, &[blocked, unblocked], kit::P0);
    assert_eq!(
        kit::power_toughness(&engine, blocked),
        (3, 3),
        "the counter makes the attacking Bear a 3/3"
    );
    kit::block(&mut engine, kit::P0, &[(hag, blocked)]);
    kit::drive(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P1)
    });

    assert!(
        in_graveyard(&engine, kit::P0, brine_hag()).is_some(),
        "the 3/3 dealt 3 to the Hag: it died"
    );
    assert!(
        kit::on_the_table(&engine, blocked),
        "2 damage on a creature that is a 1/3 is not lethal"
    );
    assert_eq!(kit::damage_on(&engine, blocked), 2, "the Hag dealt it 2");
    assert_eq!(
        kit::power_toughness(&engine, blocked),
        (1, 3),
        "base 0/2 (layer 7b) plus the +1/+1 counter (layer 7c): 1/3, not 0/2 and not 3/3"
    );
    assert_eq!(
        kit::power_toughness(&engine, unblocked),
        (2, 2),
        "the attacker that was not blocked by the Hag dealt it no damage"
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        18,
        "and its 2 damage went to the player"
    );
}

/// "Each creature that dealt damage to it" — by an ability too, and a source
/// that is not a creature does not matter. The Hag is on p1's side; Prodigal
/// Sorcerer pings it for 1, and Lightning Bolt (not a creature) finishes it. The
/// Sorcerer, which dealt it damage this turn, becomes 0/2 although it was
/// never in combat; a Bear next to it that dealt nothing stays 2/2.
#[test]
fn brine_hag_dying_counts_a_creature_that_dealt_damage_with_an_ability() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::PRODIGAL_SORCERER, kit::bears(), mountain()])
        .battlefield(1, &[brine_hag()])
        .hand(0, &[kit::bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let hag = on_battlefield(&engine, kit::P1, brine_hag()).expect("the Hag is out");
    let sorcerer =
        on_battlefield(&engine, kit::P0, index::PRODIGAL_SORCERER).expect("the Sorcerer is out");
    let bear = on_battlefield(&engine, kit::P0, kit::bears()).expect("the Bear is out");

    let mut ping = kit::Answers::new().aim(&[hag]);
    kit::activate_ability(&mut engine, kit::P0, sorcerer, 0, &mut ping);
    kit::settle(&mut engine, kit::P0, &mut ping);
    assert_eq!(kit::damage_on(&engine, hag), 1, "the ping dealt the Hag 1");
    assert!(kit::on_the_table(&engine, hag), "a 2/2 with 1 damage lives");

    tap_all_mana(&mut engine, kit::P0);
    let mut bolt = kit::Answers::new().aim(&[hag]);
    kit::cast(&mut engine, kit::P0, kit::bolt(), &mut bolt);
    kit::settle(&mut engine, kit::P0, &mut bolt);

    assert!(
        in_graveyard(&engine, kit::P1, brine_hag()).is_some(),
        "the Bolt finished it"
    );
    assert_eq!(
        kit::power_toughness(&engine, sorcerer),
        (0, 2),
        "the Sorcerer dealt the Hag damage with its ability: base 0/2"
    );
    assert_eq!(
        kit::power_toughness(&engine, bear),
        (2, 2),
        "the Bear dealt it nothing and is unchanged"
    );
}
