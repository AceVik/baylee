//! "At the beginning of the next end step" about one object
//! (`Effect::AtNextEndStep`, CR 603.7): a delayed triggered ability with the
//! creating ability's source and controller (CR 603.7e) that uses the stack,
//! and that does nothing to an object that has left its zone since, even if
//! it came back (CR 603.7c, 400.7).
//!
//! Stone Giant is the card that prints it: "{T}: Target creature you control
//! with toughness less than this creature's power gains flying until end of
//! turn. Destroy that creature at the beginning of the next end step."

use super::testkit::{
    Duel, RegistryLookup, card_index, keep_mulligans, on_battlefield, pass_until, stack_is_empty,
    walk_to_own_main,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

fn stone_giant() -> CardIndex {
    card_index("0b8e3f9b-a4da-49a3-8545-ce7a265e5856")
}

/// A 1/1: toughness 1, under the Giant's 3.
fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// The Giant and the Elves on p0's side, p0 in its first main phase, and
/// the Giant's ability resolved at the Elves.
fn thrown(seed: u64) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[stone_giant(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let giant = on_battlefield(&engine, p0, stone_giant()).expect("the Giant is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: giant,
                ability_index: 0,
            },
        )
        .expect("the Giant is untapped");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the ability asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !options.contains(&giant),
        "the Giant's own toughness 4 is not less than its power 3"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elves are a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(elf)
            .expect("still there")
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::FLYING),
        "the Elves fly until end of turn"
    );
    (engine, elf)
}

fn at_end_step(engine: &Engine<RegistryLookup>) -> bool {
    engine.state().turn.step == crate::turn::Step::End
}

/// The trigger goes on the stack at the beginning of the end step, and the
/// creature is destroyed as it resolves.
#[test]
fn the_creature_is_destroyed_by_a_trigger_at_the_next_end_step() {
    let p0 = PlayerId::new(0);
    let (mut engine, elf) = thrown(91);
    pass_until(&mut engine, at_end_step);
    assert!(
        !stack_is_empty(&engine),
        "the delayed trigger uses the stack (CR 603.7)"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and nothing is destroyed before it resolves"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(elf).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard),
        "that creature is destroyed"
    );
}

/// A creature that left the battlefield and came back is a new object, and
/// the trigger does nothing to it (CR 603.7c, 400.7).
#[test]
fn a_creature_that_left_and_came_back_is_not_destroyed() {
    let p0 = PlayerId::new(0);
    let (mut engine, elf) = thrown(93);
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    for to in [
        crate::zone::ZoneLocation::Hand(p0),
        crate::zone::ZoneLocation::Battlefield,
    ] {
        let _ = state.move_object(
            elf,
            to,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
    }
    pass_until(&mut engine, at_end_step);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(elf).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "the Elves that came back are a new object and stay"
    );
}

// --- An ability with no target: the delayed trigger is about its source --

/// Dragon Whelp: "{R}: This creature gets +1/+0 until end of turn. If this
/// ability has been activated four or more times this turn, sacrifice this
/// creature at the beginning of the next end step."
fn dragon_whelp() -> CardIndex {
    card_index("705a1985-ed39-4a4b-812e-a677170b596e")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// The Whelp and four Mountains on p0's side, p0 in its first main phase
/// with the four red floating, and the Whelp's ability activated `times`
/// times without anything resolving yet.
fn whelp_activated(seed: u64, times: usize) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, mountain())
        .battlefield(
            0,
            &[
                dragon_whelp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let whelp = on_battlefield(&engine, p0, dragon_whelp()).expect("the Whelp is seated");
    assert_eq!(super::testkit::tap_all_mana(&mut engine, p0), 4);
    for _ in 0..times {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: whelp,
                    ability_index: 0,
                },
            )
            .expect("{R} is floating");
    }
    (engine, whelp)
}

fn stack_len(engine: &Engine<RegistryLookup>) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .len()
}

/// "Has been activated four or more times" counts activations, and an
/// ability is activated once it is on the stack and paid for (CR 602.2):
/// with four stacked, the first to resolve already sees four and makes the
/// delayed sacrifice. A count of resolutions would have seen one.
#[test]
fn four_stacked_activations_are_counted_before_the_first_resolves() {
    let (mut engine, whelp) = whelp_activated(95, 4);
    assert_eq!(stack_len(&engine), 4);
    pass_until(&mut engine, |e| stack_len(e) == 3);
    let about_the_whelp = engine.state().delayed.iter().any(|d| {
        d.when == crate::state::DelayedWhen::NextEndStep
            && matches!(
                d.action,
                crate::state::DelayedAction::TriggerAbout { object, .. } if object == whelp
            )
    });
    assert!(
        about_the_whelp,
        "the first resolution made the delayed sacrifice, about the Whelp: {:?}",
        engine.state().delayed
    );
}

/// Four activations: the trigger goes on the stack at the end step, and
/// the Whelp is sacrificed as it resolves.
#[test]
fn four_activations_sacrifice_the_whelp_at_the_next_end_step() {
    let p0 = PlayerId::new(0);
    let (mut engine, whelp) = whelp_activated(97, 4);
    pass_until(&mut engine, stack_is_empty);
    pass_until(&mut engine, at_end_step);
    assert!(
        !stack_is_empty(&engine),
        "the delayed trigger uses the stack (CR 603.7)"
    );
    assert!(on_battlefield(&engine, p0, dragon_whelp()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(whelp).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard),
        "this creature is sacrificed"
    );
}

/// Three activations are not four: nothing waits for the end step.
#[test]
fn three_activations_leave_the_whelp() {
    let (mut engine, whelp) = whelp_activated(99, 3);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine.state().delayed.is_empty(),
        "{:?}",
        engine.state().delayed
    );
    pass_until(&mut engine, at_end_step);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(whelp).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield)
    );
}

/// A Whelp that left the battlefield and came back is a new object, and
/// the delayed trigger about the old one sacrifices nothing (CR 603.7c,
/// 400.7) — which a plain "sacrifice the source" would not know.
#[test]
fn a_whelp_that_left_and_came_back_is_not_sacrificed() {
    let p0 = PlayerId::new(0);
    let (mut engine, whelp) = whelp_activated(101, 4);
    pass_until(&mut engine, stack_is_empty);
    assert!(!engine.state().delayed.is_empty(), "the sacrifice waits");
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    for to in [
        crate::zone::ZoneLocation::Hand(p0),
        crate::zone::ZoneLocation::Battlefield,
    ] {
        let _ = state.move_object(
            whelp,
            to,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
    }
    pass_until(&mut engine, at_end_step);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(whelp).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "the Whelp that came back is a new object and stays"
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Follows one definition through hashing, a later text change and both source lifetimes.
fn defined_delayed_trigger_freezes_color_words_across_source_changes() {
    use crate::object::AbilityList;
    use crate::state::DelayedAction;
    use crate::text_changes::{TextChangeMap, TextReplacement};
    use baylee_cards_dsl::{Amount, Effect, Filter, TextWordKind, ZoneSel};
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::generated::index;

    static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        Cost::TAP,
        &[Effect::AtNextEndStep {
            effects: &[Effect::GainLife {
                amount: Amount::CountOf {
                    filter: &Filter::HasColor(ColorSet::of(Color::Green)),
                    zone: ZoneSel::Battlefield,
                },
            }],
        }]
    )];
    let player = PlayerId::new(0);
    for leave in [false, true] {
        let mut engine = Duel::new(185, forest())
            .battlefield(
                0,
                &[
                    forest(),
                    index::HILL_GIANT,
                    index::AIR_ELEMENTAL,
                    index::AIR_ELEMENTAL,
                ],
            )
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, player));
        let source = on_battlefield(&engine, player, forest()).unwrap();
        let identity = engine.state.source_identity(source).unwrap();
        engine
            .state
            .object_mut(source)
            .unwrap()
            .take_abilities(AbilityList::from_static(ABILITIES, None, None));
        assert!(engine.state.text_changes.replace(
            identity,
            TextReplacement {
                kind: TextWordKind::Color,
                from: Color::Green as u8,
                to: Color::Red as u8,
            }
        ));
        engine.state.invalidate_projections();
        engine
            .apply(
                player,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0,
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state.delayed.len(), 1);
        let mut unchanged_words = engine.state.clone();
        let (DelayedAction::Trigger { text, .. } | DelayedAction::TriggerAbout { text, .. }) =
            &mut unchanged_words.delayed[0].action
        else {
            panic!("defined delayed ability");
        };
        *text = TextChangeMap::IDENTITY;
        assert_ne!(
            engine.state.snapshot_hash(),
            unchanged_words.snapshot_hash(),
            "the delayed definition's words are deterministic state"
        );

        // Its printed green word now means blue, after the delayed ability
        // was defined using red. Removing the source must not erase that text.
        assert!(engine.state.text_changes.replace(
            identity,
            TextReplacement {
                kind: TextWordKind::Color,
                from: Color::Red as u8,
                to: Color::Blue as u8,
            }
        ));
        if leave {
            engine
                .state
                .move_object(
                    source,
                    ZoneLocation::Graveyard(player),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        engine.state.invalidate_projections();
        let life = engine.state.players[0].life;
        pass_until(&mut engine, at_end_step);
        assert!(
            !stack_is_empty(&engine),
            "the delayed ability still uses the stack"
        );
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state.players[0].life,
            life + 1,
            "counts the one red permanent, not two blue or zero green; left={leave}"
        );
    }
}
