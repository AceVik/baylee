//! `PlayerAction::CancelCast`: a cast begun from priority taken back in any
//! of its questions or in its payment window, and reversed as CR 732.1
//! reverses an action that is not completed. The player keeps priority
//! (CR 732.2).

use super::testkit::*;
use super::*;
use crate::zone::{Zone, ZoneLocation};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaColor;

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}
fn city_of_brass() -> CardIndex {
    card_index("f25351e3-539b-4bbc-b92d-6480acf4d722")
}
fn adarkar_wastes() -> CardIndex {
    card_index("d5ad26cc-2bdb-46b7-b8bf-dd099d5fa09b")
}
/// Crystalline Sliver, `{W}{U}`.
fn sliver() -> CardIndex {
    card_index("ba3aa1eb-722a-47d3-83be-96daddb50265")
}
fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}
/// Disintegrate, `{X}{R}`: X damage to any target.
fn disintegrate() -> CardIndex {
    card_index("92d6af2f-728e-4e41-87cb-5c90878a2f2f")
}
/// Crop Rotation, `{G}`: "As an additional cost to cast this spell,
/// sacrifice a land."
fn crop_rotation() -> CardIndex {
    card_index("28b46183-c62f-47b1-9fee-3ba148202cab")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
/// Sacrifice, `{B}` instant: "As an additional cost to cast this spell,
/// sacrifice a creature."
fn sacrifice() -> CardIndex {
    card_index("068b3692-411b-44d4-a7e9-005262760cfc")
}
/// Kazuul's Fury, `{2}{R}` instant (a land on its back): "As an additional
/// cost to cast this spell, sacrifice a creature."
fn kazuuls_fury() -> CardIndex {
    card_index("f8410804-632b-4f18-9a73-6dccc7e4582d")
}

const P0: PlayerId = PlayerId::new(0);

fn table(seed: u64, board: &[CardIndex], hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, basic_forest())
        .battlefield(0, board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn on_board(engine: &Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    on_battlefield(engine, P0, card).expect("on the battlefield")
}

fn tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
}

fn pool(engine: &Engine<RegistryLookup>) -> u64 {
    engine.state().players[0].mana_pool.total()
}

/// What a cancel must leave: the card back in hand, nothing on the stack,
/// priority with the player who was casting, and nothing cancellable.
#[track_caller]
fn cancelled(engine: &mut Engine<RegistryLookup>, card: ObjectId) {
    assert_eq!(engine.cancellable_cast(P0), Some(card), "a cast to cancel");
    engine
        .apply(P0, PlayerAction::CancelCast)
        .expect("the cast is taken back");
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(Zone::Hand),
        "the card is where it was"
    );
    assert!(engine.state().zones.stack_is_empty(), "nothing was cast");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
        "the player who was casting keeps priority (CR 732.2): {:?}",
        engine.pending()
    );
    assert_eq!(engine.cancellable_cast(P0), None);
    assert!(
        engine
            .journal()
            .entries()
            .iter()
            .rev()
            .any(|e| matches!(e.event, GameEvent::CastCancelled { card: c, .. } if c == card)),
        "the journal says so"
    );
}

/// At the target question, with the mana floated before the cast: the mana
/// was in the pool before the cast began and is in it after.
#[test]
fn a_cast_cancelled_at_its_target_keeps_the_mana_floated_before_it() {
    let mut engine = table(7320, &[mountain()], &[lightning_bolt()]);
    tap_all_mana(&mut engine, P0);
    assert_eq!(pool(&engine), 1);
    let bolt = in_hand(&engine, P0, lightning_bolt()).expect("in hand");
    engine
        .apply(P0, PlayerAction::CastSpell { card: bolt })
        .expect("castable");
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    cancelled(&mut engine, bolt);
    assert_eq!(
        pool(&engine),
        1,
        "the red floated before the cast is still there"
    );
    assert!(
        tapped(&engine, on_board(&engine, mountain())),
        "and the Mountain tapped before it stays tapped"
    );
}

/// At the X question.
#[test]
fn a_cast_cancelled_at_its_x_goes_back() {
    let mut engine = table(7321, &[mountain(), mountain()], &[disintegrate()]);
    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, disintegrate()).expect("in hand");
    engine
        .apply(P0, PlayerAction::CastSpell { card })
        .expect("castable");
    assert!(
        matches!(engine.pending(), Pending::ChooseNumber { .. }),
        "X is asked: {:?}",
        engine.pending()
    );
    cancelled(&mut engine, card);
    assert_eq!(pool(&engine), 2);
    assert_eq!(engine.state().object(card).map(|o| o.x_value), Some(0));
}

/// In the payment window (CR 601.2g) after two lands were tapped for it:
/// both are untapped and their mana is gone (CR 732.1).
#[test]
fn a_cast_cancelled_in_its_window_untaps_what_was_tapped_for_it() {
    let mut engine = table(7322, &[plains(), island()], &[sliver()]);
    let card = in_hand(&engine, P0, sliver()).expect("in hand");
    engine
        .apply(P0, PlayerAction::CastSpell { card })
        .expect("payable");
    let lands = [on_board(&engine, plains()), on_board(&engine, island())];
    for land in lands {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: land })
            .expect("a mana ability in the window");
    }
    assert_eq!(pool(&engine), 2);
    cancelled(&mut engine, card);
    assert!(lands.iter().all(|&l| !tapped(&engine, l)), "both untapped");
    assert_eq!(pool(&engine), 0, "and their mana is gone with them");
}

/// A painland tapped in the window: reversing its mana ability reverses all
/// of it, damage included, since an undone ability did nothing (CR 732.1:
/// the player "may also reverse any legal mana abilities that player
/// activated while making the illegal play"; "No abilities trigger and no
/// effects apply as a result of an undone action"). City of Brass's trigger
/// is not put on the stack for the same reason.
#[test]
fn a_cancel_reverses_a_painland_and_city_of_brass_whole() {
    let mut engine = table(7323, &[adarkar_wastes(), city_of_brass()], &[sliver()]);
    let card = in_hand(&engine, P0, sliver()).expect("in hand");
    let life = engine.state().players[0].life;
    engine
        .apply(P0, PlayerAction::CastSpell { card })
        .expect("payable");
    let (wastes, city) = (
        on_board(&engine, adarkar_wastes()),
        on_board(&engine, city_of_brass()),
    );
    for (land, color) in [(wastes, ManaColor::Blue), (city, ManaColor::White)] {
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 1,
                },
            )
            .expect("the coloured line");
        engine
            .apply(P0, PlayerAction::ChooseColor(color))
            .expect("offered");
    }
    assert_eq!(
        engine.state().players[0].life,
        life - 1,
        "the Wastes' damage"
    );
    cancelled(&mut engine, card);
    assert_eq!(
        engine.state().players[0].life,
        life,
        "undone with its ability"
    );
    assert!(!tapped(&engine, wastes) && !tapped(&engine, city));
    assert_eq!(pool(&engine), 0);
    pass_until(&mut engine, |e| e.state().turn.active != P0);
    assert_eq!(
        engine.state().players[0].life,
        life,
        "the City's trigger never went on the stack"
    );
}

/// A window passed short is the same reversal as a cancel.
#[test]
fn a_window_passed_short_is_reversed_like_a_cancel() {
    let mut engine = table(7324, &[adarkar_wastes()], &[sliver()]);
    let card = in_hand(&engine, P0, sliver()).expect("in hand");
    let life = engine.state().players[0].life;
    engine
        .apply(P0, PlayerAction::CastSpell { card })
        .expect("payable");
    let wastes = on_board(&engine, adarkar_wastes());
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: wastes,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(Zone::Hand)
    );
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0));
    assert_eq!(engine.state().players[0].life, life);
    assert!(!tapped(&engine, wastes));
    assert_eq!(pool(&engine), 0);
}

/// An extra cost chosen in the cast is not paid: Crop Rotation's land,
/// chosen and then the cast taken back in its window, is still on the
/// battlefield, and the Forest tapped for it is untapped.
#[test]
fn a_cancel_leaves_the_land_chosen_for_an_extra_cost() {
    let mut engine = table(7325, &[basic_forest(), plains()], &[crop_rotation()]);
    let card = in_hand(&engine, P0, crop_rotation()).expect("in hand");
    let (forest_id, land) = (
        on_board(&engine, basic_forest()),
        on_board(&engine, plains()),
    );
    engine
        .apply(P0, PlayerAction::CastSpell { card })
        .expect("payable");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the land to sacrifice is asked: {:?}", engine.pending())
    };
    assert!(options.contains(&land), "{options:?}");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("the Plains is offered");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the payment window: {:?}",
        engine.pending()
    );
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: forest_id })
        .expect("the Forest in the window");
    cancelled(&mut engine, card);
    assert_eq!(
        engine.state().object(land).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the Plains was never sacrificed"
    );
    assert!(!tapped(&engine, forest_id));
}

/// The creature chosen for Sacrifice's extra cost, then the cast taken back
/// in its window: the creature never left the battlefield.
#[test]
fn a_cancel_leaves_the_creature_chosen_for_an_extra_cost() {
    let mut engine = table(7327, &[swamp(), quiet_creature()], &[sacrifice()]);
    let card = in_hand(&engine, P0, sacrifice()).expect("in hand");
    let creature = on_board(&engine, quiet_creature());
    engine
        .apply(P0, PlayerAction::CastSpell { card })
        .expect("payable");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the creature to sacrifice is asked: {:?}", engine.pending())
    };
    assert_eq!(options, [creature]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .expect("the creature is offered");
    let swamp_id = on_board(&engine, swamp());
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: swamp_id })
        .expect("the Swamp in the window");
    cancelled(&mut engine, card);
    assert_eq!(
        engine.state().object(creature).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the creature was never sacrificed"
    );
    assert!(!tapped(&engine, swamp_id));
}

/// Every instant in the pool with an extra cost to choose (Sacrifice, Crop
/// Rotation, Kazuul's Fury) may be cast at instant speed, on the
/// opponent's turn, paying in the window. Sacrifice was reported refused
/// for its timing (08.10.2026); the report was wrong: the probe that said
/// so had been handed a Swamp's oracle id for the card's.
#[test]
fn every_instant_with_an_extra_cost_is_cast_on_the_opponents_turn() {
    let cases = [
        (sacrifice(), vec![swamp(), quiet_creature()]),
        (crop_rotation(), vec![basic_forest(), plains()]),
        (
            kazuuls_fury(),
            vec![mountain(), mountain(), mountain(), quiet_creature()],
        ),
    ];
    for (spell, board) in cases {
        let mut engine = table(7328, &board, &[spell]);
        reach_their_main_phase(&mut engine, PlayerId::new(1));
        let card = in_hand(&engine, P0, spell).expect("in hand");
        assert_eq!(
            crate::casting::can_cast_paying_later(engine.state(), &RegistryLookup, P0, card),
            Ok(()),
            "{spell:?} at instant speed"
        );
        assert_eq!(
            crate::casting::can_cast(engine.state(), &RegistryLookup, P0, card),
            Err(crate::casting::CastError::NotEnoughMana),
            "{spell:?}: only the mana is missing"
        );
    }
}

/// Nothing is being cast: there is nothing to cancel, and no other question
/// takes it.
#[test]
fn there_is_no_cancel_without_a_cast() {
    let mut engine = table(7326, &[mountain()], &[lightning_bolt()]);
    assert_eq!(engine.cancellable_cast(P0), None);
    assert!(engine.apply(P0, PlayerAction::CancelCast).is_err());
    assert!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len() == 1,
        "nothing moved"
    );
}
