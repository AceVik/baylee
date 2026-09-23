//! Regeneration (CR 701.19), which is a replacement effect and not a saving
//! throw.
//!
//! Seven cards in this pool print it and every one of them was
//! `Coverage::Partial` for the same missing sentence. The rule is one
//! shield count on the permanent and one branch in `sba::destroy`, and
//! these are the tests that the branch is where the rules put it: it
//! replaces **destruction**, so it catches "destroy target creature" and
//! lethal damage alike, and it catches neither of the state-based actions
//! that put a permanent into a graveyard without destroying it.
//!
//! The population is deliberately the rule and not a card. Each of the
//! seven has its own test beside its own file; what cannot be asked there
//! is whether the *next* card to print it will behave, and that is what a
//! module named after the rule is for.

use super::testkit::{
    Duel, RegistryLookup, card_index, keep_mulligans, on_battlefield, pt, tap_mana_except,
    walk_to_own_main,
};
use super::*;
use crate::object::Status;
use baylee_core::ids::{CardIndex, ObjectId};

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// `{G}, {T}: Regenerate target creature.`
fn yavimaya_hollow() -> CardIndex {
    card_index("53d6113d-acdb-4754-9641-f7991a96c7b9")
}

/// A 1/1 that does nothing but make mana, so nothing on the board reads a
/// destruction but the rule under test.
fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// `{1}{G}: Regenerate Thrun.` — a price with no tap symbol in it.
fn thrun_the_last_troll() -> CardIndex {
    card_index("1149e5ac-554a-41b1-84ae-bac42579c1aa")
}

/// A 6/6, for the boards where a 1/1 would die to its own arithmetic.
fn rootbreaker_wurm() -> CardIndex {
    card_index("d3edbb47-6892-4853-badc-cc01499d4e55")
}

/// A board with the Hollow, enough Forests to use it `shields` times, and a
/// creature to point it at.
fn shielded(seed: u64, shields: usize) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut board = vec![yavimaya_hollow(), llanowar_elves()];
    board.extend(std::iter::repeat_n(forest(), shields));
    let mut engine = Duel::new(seed, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let hollow = on_battlefield(&engine, p0, yavimaya_hollow()).expect("the Hollow is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    (engine, hollow, elf)
}

/// Activates the Hollow at `creature`, floating the mana for it first.
#[track_caller]
fn regenerate(engine: &mut Engine<RegistryLookup>, hollow: ObjectId, creature: ObjectId) {
    let p0 = PlayerId::new(0);
    // Everything but the Hollow: its own `{T}` is half the price.
    tap_mana_except(engine, p0, hollow);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: hollow,
                ability_index: 1,
            },
        )
        .expect("{G} is floating and the land is untapped");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the ability asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&creature),
        "the creature this test regenerates is a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .expect("the creature came out of the menu");
    super::testkit::pass_until(engine, super::testkit::stack_is_empty);
}

/// Passes priority once, which is what makes state-based actions run.
///
/// `Engine::refresh_offer` recomputes the offer and nothing else — the
/// state-based fixpoint runs before a priority *grant* (CR 704.3), so a
/// board rewritten through `dev_state_mut` has not been judged until
/// somebody passes.
#[track_caller]
fn settle(engine: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::PassPriority)
        .expect("passing priority is always legal");
}

/// Destroys `id` the way a card would, without spending a turn casting one.
///
/// `sba::destroy` is the one door every destruction in this engine goes
/// through, so calling it is the same event Terminate and lethal damage
/// produce — which is the property under test rather than a shortcut around
/// it.
#[track_caller]
fn destroy(engine: &mut Engine<RegistryLookup>, id: ObjectId) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    crate::sba::destroy(state, id);
}

/// CR 701.19a in one board: the shield replaces the destruction, and it is
/// spent doing so.
///
/// Both halves matter and neither is the other. A shield that saved nothing
/// would fail the first assertion; a shield that never ran out would fail
/// the second, and a rule written as a keyword bit rather than a count is
/// exactly the rule that never runs out.
#[test]
fn a_shield_replaces_one_destruction_and_is_spent_on_it() {
    let p0 = PlayerId::new(0);
    let (mut engine, hollow, elf) = shielded(71, 1);
    regenerate(&mut engine, hollow, elf);

    destroy(&mut engine, elf);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the first destruction is replaced: the creature is still there"
    );

    destroy(&mut engine, elf);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and the second one kills it, because the shield was spent"
    );
}

/// What the replacement actually does (CR 701.19a): tap it, remove all
/// damage marked on it, and take it out of combat.
///
/// The tap is asserted from the projection rather than the bit, because
/// `sba` routes it through `GameState::set_tapped` for exactly that reason
/// — a bit written by hand would leave every continuous effect reading
/// "untapped" while the board says otherwise.
#[test]
fn a_regenerated_creature_is_tapped_and_its_damage_removed() {
    let p0 = PlayerId::new(0);
    let (mut engine, hollow, wurm) = {
        let mut board = vec![yavimaya_hollow(), rootbreaker_wurm(), forest()];
        board.push(forest());
        let mut engine = Duel::new(72, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        let hollow = on_battlefield(&engine, p0, yavimaya_hollow()).expect("the Hollow is seated");
        let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is seated");
        (engine, hollow, wurm)
    };
    // Three points on a 6/6: not lethal, so nothing has happened yet.
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    state
        .object_mut(wurm)
        .expect("the Wurm is an object")
        .damage = 3;

    regenerate(&mut engine, hollow, wurm);
    assert!(
        !engine
            .state()
            .object(wurm)
            .expect("still there")
            .status
            .contains(Status::TAPPED),
        "the shield does nothing at all until something would destroy it"
    );

    destroy(&mut engine, wurm);
    let obj = engine.state().object(wurm).expect("it survived");
    assert!(
        obj.status.contains(Status::TAPPED),
        "its controller taps it"
    );
    assert_eq!(obj.damage, 0, "and all damage marked on it is removed");
    assert_eq!(pt(&engine, wurm), (6, 6), "the body is untouched");
}

/// A shield is a count, not a flag.
///
/// Two activations survive two destructions and the third one kills, which
/// is the assertion a `bool` cannot pass. Thrun and not Yavimaya Hollow,
/// because the Hollow's own `{T}` is half its price and a land taps once a
/// turn — and the shield would be gone by the next one. Thrun's line costs
/// mana and nothing else, so both activations happen while the first
/// shield is still standing.
#[test]
fn two_shields_survive_two_destructions_and_not_a_third() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(73, forest())
        .battlefield(
            0,
            &[
                thrun_the_last_troll(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let thrun = on_battlefield(&engine, p0, thrun_the_last_troll()).expect("Thrun is seated");

    super::testkit::tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two activations' worth of {{1}}{{G}}"
    );
    for _ in 0..2 {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: thrun,
                    ability_index: 0,
                },
            )
            .expect("{1}{G} is floating");
        super::testkit::pass_until(&mut engine, super::testkit::stack_is_empty);
    }
    assert_eq!(
        engine
            .state()
            .object(thrun)
            .expect("still there")
            .regeneration_shields,
        2,
        "two activations, two shields"
    );

    destroy(&mut engine, thrun);
    destroy(&mut engine, thrun);
    assert!(
        on_battlefield(&engine, p0, thrun_the_last_troll()).is_some(),
        "both destructions are replaced"
    );
    destroy(&mut engine, thrun);
    assert!(
        on_battlefield(&engine, p0, thrun_the_last_troll()).is_none(),
        "and the third kills it"
    );
}

/// "The next time it would be destroyed **this turn**" (CR 701.19a): an
/// unspent shield does not keep.
#[test]
fn a_shield_does_not_survive_the_cleanup_step() {
    let p0 = PlayerId::new(0);
    let (mut engine, hollow, elf) = shielded(74, 1);
    regenerate(&mut engine, hollow, elf);

    super::testkit::pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        engine
            .state()
            .object(elf)
            .expect("still there")
            .regeneration_shields,
        0,
        "the cleanup step took it"
    );
    destroy(&mut engine, elf);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "so the same destruction that was replaced last turn kills it now"
    );
}

/// Lethal damage is destruction (CR 704.5g), so the shield catches it — and
/// the creature stands there tapped with the damage gone rather than dying
/// to the next state-based check.
///
/// The second half is the one a naive implementation fails: clearing the
/// damage is not decoration, it is what stops the fixpoint from spending
/// every shield in one pass and killing the creature anyway.
#[test]
fn lethal_damage_is_a_destruction_the_shield_replaces() {
    let p0 = PlayerId::new(0);
    let (mut engine, hollow, elf) = shielded(75, 1);
    regenerate(&mut engine, hollow, elf);

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    state.object_mut(elf).expect("the Elf is an object").damage = 5;
    settle(&mut engine);

    let obj = engine.state().object(elf).expect("it survived the bolt");
    assert!(obj.status.contains(Status::TAPPED), "tapped by the shield");
    assert_eq!(
        obj.damage, 0,
        "and undamaged, so the next check finds nothing"
    );
    assert_eq!(
        obj.regeneration_shields, 0,
        "one shield, spent once — not once per state-based pass"
    );
}

/// Zero toughness is **not** destruction (CR 704.5f), so a shield does not
/// answer it.
///
/// The pair beside it is what makes this a statement about the rule rather
/// than about the board: the same creature with the same shield survives a
/// destruction on the line above.
#[test]
fn a_shield_does_not_answer_zero_toughness() {
    let p0 = PlayerId::new(0);
    let (mut engine, hollow, elf) = shielded(76, 1);
    regenerate(&mut engine, hollow, elf);

    // A -1/-1 counter on the 1/1, which is CR 704.5f's own arithmetic and
    // not a destruction anybody performed.
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    state
        .object_mut(elf)
        .expect("the Elf is an object")
        .counters
        .add(crate::object::CounterKind::M1M1, 1);
    state.invalidate_projections();
    settle(&mut engine);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "a 1/0 goes to the graveyard however many shields it has: CR 704.5f \
         is not destruction and indestructible does not answer it either"
    );
}

/// "It can't be regenerated" (CR 701.19c) does not stop a shield being
/// created — it stops one being *applied*.
///
/// The distinction is the whole of the rule and it is observable: the
/// shield is still standing before the spell, the spell kills through it,
/// and the eight cards in this pool that print the clause were correct for
/// free while no shield existed at all.
#[test]
fn a_destruction_that_says_so_kills_through_a_shield() {
    let p0 = PlayerId::new(0);
    let (mut engine, hollow, elf) = shielded(77, 1);
    regenerate(&mut engine, hollow, elf);
    assert_eq!(
        engine
            .state()
            .object(elf)
            .expect("still there")
            .regeneration_shields,
        1,
        "the shield is there to be ignored"
    );

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::sba::destroy_no_regen(state, elf);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "Terminate's clause goes through the shield"
    );
}

/// "If it's an attacking or blocking creature, remove it from combat"
/// (CR 701.19a) — the third of the three things the replacement does, and
/// the one no board without a combat can show.
///
/// Thrun rather than the Hollow, because the Hollow's price is paid by
/// tapping every mana source on the board and a tapped creature cannot
/// attack.
#[test]
fn a_regenerated_attacker_leaves_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(78, forest())
        .battlefield(0, &[thrun_the_last_troll(), forest(), forest()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let thrun = on_battlefield(&engine, p0, thrun_the_last_troll()).expect("Thrun is seated");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the 6/6 is seated");

    super::testkit::tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: thrun,
                ability_index: 0,
            },
        )
        .expect("{1}{G} is floating");
    super::testkit::pass_until(&mut engine, super::testkit::stack_is_empty);

    super::testkit::pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(thrun, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .expect("a 4/4 with a shield may attack");
    super::testkit::pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wurm, thrun)],
            },
        )
        .expect("the 6/6 blocks");

    // Stop on the **shield being spent**, not on the attacker list
    // emptying: that list empties by itself at end of combat, so a loop
    // waiting for it would run past the replacement and then find an empty
    // list whether or not anything had been removed from combat — which is
    // the one claim this test exists to make.
    super::testkit::pass_until(&mut engine, |e| {
        e.state()
            .object(thrun)
            .is_none_or(|o| o.regeneration_shields == 0)
    });
    assert_eq!(
        engine.state().turn.step,
        crate::turn::Step::CombatDamage,
        "the shield was spent in the combat damage step, which is what makes \
         the next assertion about regeneration rather than about combat ending"
    );

    let obj = engine
        .state()
        .object(thrun)
        .expect("six damage on a 4/4 with a shield is a creature that is still there");
    assert_eq!(obj.damage, 0, "all damage marked on it is removed");
    assert_eq!(obj.regeneration_shields, 0, "the shield paid for that");
    assert!(
        !engine
            .state()
            .combat
            .attackers
            .iter()
            .any(|a| a.creature == thrun),
        "and it is out of combat while the combat is still going on"
    );
    assert_eq!(
        engine
            .state()
            .object(wurm)
            .expect("the 6/6 blocked and lived")
            .damage,
        4,
        "the blocker took the Troll's four, so combat damage did happen"
    );
}
