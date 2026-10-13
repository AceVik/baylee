//! `cards/instants/mv_1/remove_enchantments.rs`, played.
//!
//! Oracle: "Return to your hand all enchantments you both own and control,
//! all Auras you own attached to permanents you control, and all Auras you own
//! attached to attacking creatures your opponents control. Then destroy all
//! other enchantments you control, all other Auras attached to permanents you
//! control, and all other Auras attached to attacking creatures your
//! opponents control."
//!
//! The card has no Scryfall rulings. Written from the Oracle text and the
//! Comprehensive Rules (CR 701.8a, destroy; CR 303.4e, an Aura's controller
//! is separate from the enchanted object's; CR 508.1k, an attacking creature
//! is one that was declared as an attacker).
//!
//! One scene, three readings of it. It is seat 1's turn and seat 1's Grizzly
//! Bears attack seat 0; seat 0 casts the spell in the declare-attackers step.
//! Everything is placed by playing it: Auras are cast, aimed and resolved, and
//! only the stolen Crusade is a fixture (a change of control, so an enchantment
//! is controlled by one player and owned by the other).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::aura_kit as kit;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

// oracle_id = "fa879d99-3d2b-4a9a-a17f-d5ac109f8f44"
fn remove_enchantments() -> CardIndex {
    card_index("fa879d99-3d2b-4a9a-a17f-d5ac109f8f44")
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const W: [(ManaColor, u32); 1] = [(ManaColor::White, 1)];

/// Everything in the scene, named by whose it is and where it is.
struct Scene {
    engine: Engine<RegistryLookup>,
    /// Seat 0's creature, in play from the start.
    mine_creature: ObjectId,
    /// Seat 1's attacking Grizzly Bears.
    attacker: ObjectId,
    /// Seat 1's Hill Giant, which stays home.
    homebody: ObjectId,
    /// A Crusade seat 0 owns and controls.
    my_crusade: ObjectId,
    /// A Crusade seat 1 owns and seat 0 controls.
    stolen_crusade: ObjectId,
    /// A Crusade seat 1 owns and controls.
    their_crusade: ObjectId,
    /// Holy Strengths seat 0 owns, on its own creature, on the attacker, on the homebody.
    my_aura_at_home: ObjectId,
    my_aura_on_attacker: ObjectId,
    my_aura_on_homebody: ObjectId,
    /// Holy Strengths seat 1 owns, on seat 0's creature, on the attacker, on the homebody.
    their_aura_on_mine: ObjectId,
    their_aura_on_attacker: ObjectId,
    their_aura_on_homebody: ObjectId,
}

/// The scene is what it says before anything is cast: every Aura on the
/// creature it was aimed at, and seat 1's Bears attacking.
#[track_caller]
fn assert_the_scene_is_as_built(scene: &Scene) {
    for (aura, host) in [
        (scene.my_aura_at_home, scene.mine_creature),
        (scene.my_aura_on_attacker, scene.attacker),
        (scene.my_aura_on_homebody, scene.homebody),
        (scene.their_aura_on_mine, scene.mine_creature),
        (scene.their_aura_on_attacker, scene.attacker),
        (scene.their_aura_on_homebody, scene.homebody),
    ] {
        assert_eq!(
            kit::attached_to(&scene.engine, aura),
            Some(host),
            "{aura:?} starts on {host:?}"
        );
    }
    assert!(
        scene.engine.state().combat.is_attacking(scene.attacker),
        "seat 1's Bears are an attacking creature"
    );
}

/// Builds the scene, up to seat 0 holding priority in the declare-attackers
/// step, and (when `cast_the_spell`) has it cast Remove Enchantments and
/// resolve. Without the cast it is the same board one step earlier, the
/// control for what the spell then changes.
#[track_caller]
fn scene(cast_the_spell: bool) -> Scene {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::CRUSADE],
        &[
            index::GRIZZLY_BEARS,
            index::HILL_GIANT,
            index::CRUSADE,
            index::CRUSADE,
        ],
        &[
            index::HOLY_STRENGTH,
            index::HOLY_STRENGTH,
            index::HOLY_STRENGTH,
            remove_enchantments(),
        ],
        &[
            index::HOLY_STRENGTH,
            index::HOLY_STRENGTH,
            index::HOLY_STRENGTH,
        ],
    );
    let mine_creature = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let my_crusade = lk::obj(&engine, P0, index::CRUSADE);
    let attacker = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let homebody = lk::obj(&engine, P1, index::HILL_GIANT);
    let crusades = all_on_battlefield(&engine, P1, index::CRUSADE);
    assert_eq!(crusades.len(), 2, "seat 1 has two Crusades to start");
    let (their_crusade, stolen_crusade) = (crusades[0], crusades[1]);

    // Seat 0's turn: its three Auras, one on each of three creatures.
    let my_aura_at_home = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, mine_creature, &W);
    let my_aura_on_attacker = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, attacker, &W);
    let my_aura_on_homebody = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, homebody, &W);
    // One of seat 1's Crusades is seat 0's to control, not to own.
    crate::resolve::gain_control(
        engine.dev_state_mut(P0).expect("the harness may set up"),
        &[(stolen_crusade, P0)],
    );
    engine.refresh_offer();
    assert_eq!(kit::controller_of(&engine, stolen_crusade), P0);
    assert_eq!(kit::owner_of(&engine, stolen_crusade), P1);

    // Seat 1's turn: its three Auras, then the attack.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    let their_aura_on_mine =
        kit::cast_aura(&mut engine, P1, index::HOLY_STRENGTH, mine_creature, &W);
    let their_aura_on_attacker =
        kit::cast_aura(&mut engine, P1, index::HOLY_STRENGTH, attacker, &W);
    let their_aura_on_homebody =
        kit::cast_aura(&mut engine, P1, index::HOLY_STRENGTH, homebody, &W);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P0))],
            },
        )
        .expect("the Bears attack seat 0");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });

    let scene = Scene {
        engine,
        mine_creature,
        attacker,
        homebody,
        my_crusade,
        stolen_crusade,
        their_crusade,
        my_aura_at_home,
        my_aura_on_attacker,
        my_aura_on_homebody,
        their_aura_on_mine,
        their_aura_on_attacker,
        their_aura_on_homebody,
    };
    assert_the_scene_is_as_built(&scene);
    let mut scene = scene;
    if cast_the_spell {
        lk::float(&mut scene.engine, P0, &W);
        cast_with_floating(&mut scene.engine, P0, remove_enchantments());
        assert!(
            !matches!(scene.engine.pending(), Pending::ChooseTargets { .. }),
            "it targets nothing: {:?}",
            scene.engine.pending()
        );
        pass_until(&mut scene.engine, stack_is_empty);
    }
    scene
}

/// "Return to your hand all enchantments you both own and control, all Auras
/// you own attached to permanents you control, and all Auras you own attached
/// to attacking creatures your opponents control." Seat 0's own Crusade, its
/// Holy Strength on its own creature and its Holy Strength on the opposing
/// attacker are back in its hand, still its cards. The control: before the
/// spell they were all on the battlefield.
#[test]
fn what_you_own_comes_back_to_your_hand() {
    let before = scene(false);
    for (name, id) in [
        ("seat 0's own Crusade", before.my_crusade),
        ("seat 0's Aura on its own creature", before.my_aura_at_home),
        ("seat 0's Aura on the attacker", before.my_aura_on_attacker),
    ] {
        assert_eq!(
            kit::zone_now(&before.engine, id),
            Zone::Battlefield,
            "control: {name} is in play before the spell"
        );
    }

    let after = scene(true);
    for (name, id) in [
        ("an enchantment you own and control", after.my_crusade),
        (
            "an Aura you own on a permanent you control",
            after.my_aura_at_home,
        ),
        (
            "an Aura you own on an attacking opposing creature",
            after.my_aura_on_attacker,
        ),
    ] {
        assert_eq!(
            kit::zone_now(&after.engine, id),
            Zone::Hand,
            "{name} returns to hand"
        );
        assert_eq!(
            kit::owner_of(&after.engine, id),
            P0,
            "and is still seat 0's card"
        );
    }
}

/// "Then destroy all other enchantments you control, all other Auras attached
/// to permanents you control, and all other Auras attached to attacking
/// creatures your opponents control." Seat 1's Crusade that seat 0 controls,
/// seat 1's Holy Strength on seat 0's creature and seat 1's Holy Strength on
/// its own attacker are destroyed, which puts them in seat 1's graveyard.
#[test]
fn the_other_enchantments_and_auras_are_destroyed() {
    let after = scene(true);
    for (name, id) in [
        (
            "an enchantment you control and don't own",
            after.stolen_crusade,
        ),
        (
            "an Aura you don't own on a permanent you control",
            after.their_aura_on_mine,
        ),
        (
            "an Aura you don't own on an attacking opposing creature",
            after.their_aura_on_attacker,
        ),
    ] {
        assert_eq!(
            kit::zone_now(&after.engine, id),
            Zone::Graveyard,
            "{name} is destroyed"
        );
        assert_eq!(
            kit::owner_of(&after.engine, id),
            P1,
            "into its owner's graveyard"
        );
    }
}

/// And nothing else: an Aura on an opposing creature that is not attacking
/// (seat 1's own and seat 0's alike) and an enchantment seat 1 owns and
/// controls are untouched, and so are the creatures. The control is that the
/// two Holy Strengths on the attacking creature, which is in the same scene,
/// did leave.
#[test]
fn the_rest_is_left_alone() {
    let after = scene(true);
    assert!(
        !kit::on_the_table(&after.engine, after.my_aura_on_attacker)
            && !kit::on_the_table(&after.engine, after.their_aura_on_attacker),
        "control: both Auras on the attacker were taken"
    );
    for (name, id, host) in [
        (
            "an Aura you own on a creature that is not attacking",
            after.my_aura_on_homebody,
            Some(after.homebody),
        ),
        (
            "an opponent's Aura on its own creature that is not attacking",
            after.their_aura_on_homebody,
            Some(after.homebody),
        ),
        ("an opponent's own enchantment", after.their_crusade, None),
    ] {
        assert_eq!(
            kit::zone_now(&after.engine, id),
            Zone::Battlefield,
            "{name} stays in play"
        );
        assert_eq!(
            kit::attached_to(&after.engine, id),
            host,
            "{name}: where it was"
        );
    }
    for creature in [after.mine_creature, after.attacker, after.homebody] {
        assert!(
            kit::on_the_table(&after.engine, creature),
            "no creature was touched: {creature:?}"
        );
    }
    assert_eq!(
        kit::controller_of(&after.engine, after.their_crusade),
        P1,
        "and it is still seat 1's"
    );
}
