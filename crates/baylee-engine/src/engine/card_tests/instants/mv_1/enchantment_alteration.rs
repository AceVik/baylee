//! `cards/instants/mv_1/enchantment_alteration.rs`, played.
//!
//! Oracle: "Attach target Aura attached to a creature or land to another
//! permanent of that type."
//!
//! Written from that text, the two Scryfall rulings (only the Aura is
//! targeted; the new host is not chosen when the spell is cast) and the
//! Comprehensive Rules: CR 701.3a-b (an Aura can't be attached to what it
//! couldn't enchant; an attach that can't be done leaves it where it is),
//! CR 303.4e (an Aura's controller is separate from the enchanted object's),
//! CR 303.4j.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::aura_kit as kit;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

// oracle_id = "c80935bf-c17f-4940-ad9f-3f4e3e8f71bc"
fn enchantment_alteration() -> CardIndex {
    card_index("c80935bf-c17f-4940-ad9f-3f4e3e8f71bc")
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const W: [(ManaColor, u32); 1] = [(ManaColor::White, 1)];
const G: [(ManaColor, u32); 1] = [(ManaColor::Green, 1)];

fn sorted(mut ids: Vec<ObjectId>) -> Vec<ObjectId> {
    ids.sort_unstable();
    ids
}

/// `caster` puts `{U}` in the pool, casts Enchantment Alteration and aims it
/// at `aura`. The target menu is returned (and the spell is left on the stack
/// with its target chosen) so a test can read what was and was not on it.
#[track_caller]
fn cast_at_aura(
    engine: &mut Engine<RegistryLookup>,
    caster: PlayerId,
    aura: ObjectId,
) -> Vec<ObjectId> {
    lk::float(engine, caster, &[(ManaColor::Blue, 1)]);
    cast_with_floating(engine, caster, enchantment_alteration());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the spell is asked for its one target, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, caster, "the caster aims it");
    lk::aim(engine, &[aura], &[]);
    options
}

/// Lets the spell resolve. When it asks where the Aura goes the answer is
/// `pick` (the test says which, or `None` to say it expects no question) and
/// the menu it was asked from is returned; when the engine settles the
/// choice itself (a single legal host, or none) the result is `None`.
#[track_caller]
fn resolve(engine: &mut Engine<RegistryLookup>, pick: Option<ObjectId>) -> Option<Vec<ObjectId>> {
    pass_until(engine, |e| {
        kit::asks_for_a_permanent(e) || stack_is_empty(e)
    });
    let menu = if kit::asks_for_a_permanent(engine) {
        let pick = pick.expect("this scenario expects no question about where the Aura goes");
        Some(kit::choose_among(engine, pick))
    } else {
        None
    };
    pass_until(engine, stack_is_empty);
    menu
}

/// "Attach target Aura attached to a creature … to another permanent of that
/// type." A Holy Strength on seat 0's Grizzly Bears is moved to the opponent's
/// Wall of Wood: the creatures it may go to are every creature but the one it
/// is on (the Giant and the Wall, and not the Aura, the Bears, a land or an
/// artifact), the +1/+2 follows it, and the Aura is still seat 0's (CR 303.4e).
/// The spell is spent: it resolved.
#[test]
fn an_aura_on_a_creature_moves_to_another_creature_even_the_opponents() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[index::WALL_OF_WOOD],
        &[index::HOLY_STRENGTH, enchantment_alteration()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let wood = lk::obj(&engine, P1, index::WALL_OF_WOOD);
    let strength = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, bears, &W);
    assert_eq!(pt(&engine, bears), (3, 4), "a 2/2 with +1/+2");

    let targets = cast_at_aura(&mut engine, P0, strength);
    assert!(
        targets.contains(&strength),
        "the Aura on a creature is a legal target: {targets:?}"
    );
    let menu = resolve(&mut engine, Some(wood)).expect("two other creatures: it asks");
    assert_eq!(
        sorted(menu),
        sorted(vec![giant, wood]),
        "every creature but the one the Aura is on"
    );

    assert_eq!(kit::attached_to(&engine, strength), Some(wood));
    assert_eq!(
        pt(&engine, wood),
        (1, 5),
        "the Wall of Wood is a 0/3 with +1/+2"
    );
    assert_eq!(pt(&engine, bears), (2, 2), "the Bears lost it");
    assert_eq!(
        kit::controller_of(&engine, strength),
        P0,
        "the Aura stays under its controller's control (CR 303.4e)"
    );
    assert!(
        kit::on_the_table(&engine, strength),
        "and stays on the battlefield"
    );
    assert!(
        in_graveyard(&engine, P0, enchantment_alteration()).is_some(),
        "the instant resolved and went to the graveyard"
    );
}

/// "… attached to a creature or land to another permanent of that type": an
/// Aura on a land goes to another land, and only a land. Seat 0's Wild Growth
/// on a Forest is offered the other Forest and the opponent's Mountain; the
/// Grizzly Bears are not on the menu.
#[test]
fn an_aura_on_a_land_moves_to_another_land_and_only_a_land() {
    let mut engine = lk::game(
        SEED,
        &[index::FOREST, index::FOREST, index::GRIZZLY_BEARS],
        &[index::MOUNTAIN],
        &[index::WILD_GROWTH, enchantment_alteration()],
        &[],
    );
    let forests = all_on_battlefield(&engine, P0, index::FOREST);
    assert_eq!(forests.len(), 2, "two Forests");
    let mountain = lk::obj(&engine, P1, index::MOUNTAIN);
    let growth = kit::cast_aura(&mut engine, P0, index::WILD_GROWTH, forests[0], &G);

    let targets = cast_at_aura(&mut engine, P0, growth);
    assert!(
        targets.contains(&growth),
        "the Aura on a land is a legal target: {targets:?}"
    );
    let menu = resolve(&mut engine, Some(mountain)).expect("two other lands: it asks");
    assert_eq!(
        sorted(menu),
        sorted(vec![forests[1], mountain]),
        "every land but the one it is on, and no creature"
    );
    assert_eq!(kit::attached_to(&engine, growth), Some(mountain));
}

/// The spell's target: Auras on creatures and on lands, and nothing else. An
/// Aura on an artifact (Living Artifact on a plain artifact) is not offered,
/// and neither are the permanents the Auras are on.
#[test]
fn only_auras_on_creatures_and_lands_can_be_aimed_at() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::FOREST, quiet_artifact()],
        &[],
        &[
            index::HOLY_STRENGTH,
            index::WILD_GROWTH,
            index::LIVING_ARTIFACT,
            enchantment_alteration(),
        ],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let forest = lk::obj(&engine, P0, index::FOREST);
    let artifact = lk::obj(&engine, P0, quiet_artifact());
    let strength = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, bears, &W);
    let growth = kit::cast_aura(&mut engine, P0, index::WILD_GROWTH, forest, &G);
    let living = kit::cast_aura(&mut engine, P0, index::LIVING_ARTIFACT, artifact, &G);

    let targets = cast_at_aura(&mut engine, P0, strength);
    assert!(targets.contains(&strength), "on a creature: {targets:?}");
    assert!(targets.contains(&growth), "on a land: {targets:?}");
    assert!(
        !targets.contains(&living),
        "on an artifact that is neither: {targets:?}"
    );
    for not_an_aura in [bears, forest, artifact] {
        assert!(
            !targets.contains(&not_an_aura),
            "{not_an_aura:?} is not an Aura: {targets:?}"
        );
    }
}

/// Ruling: "Only targets the Aura and not either of the things the Aura may be
/// on." Seat 1's Sylvan Caryatid has hexproof, and seat 0 moves its own Holy
/// Strength onto it anyway: the new host is chosen, not targeted.
#[test]
fn the_new_host_is_not_targeted_so_hexproof_does_not_stop_it() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[index::SYLVAN_CARYATID],
        &[index::HOLY_STRENGTH, enchantment_alteration()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let caryatid = lk::obj(&engine, P1, index::SYLVAN_CARYATID);
    let strength = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, bears, &W);

    cast_at_aura(&mut engine, P0, strength);
    let menu = resolve(&mut engine, Some(caryatid)).expect("two other creatures: it asks");
    assert_eq!(
        sorted(menu),
        sorted(vec![giant, caryatid]),
        "the hexproof creature is on the menu: nothing is targeting it"
    );
    assert_eq!(
        kit::attached_to(&engine, strength),
        Some(caryatid),
        "the Aura went onto the hexproof creature"
    );
}

/// The other half of the same ruling: the Aura sits on a hexproof creature
/// (seat 0's own Caryatid), and the *opponent's* Enchantment Alteration still
/// aims at it, because the spell targets the Aura and the Aura has no
/// hexproof. Seat 1 chooses where it goes (its own Bears), and it remains
/// seat 0's Aura (CR 303.4e).
#[test]
fn an_aura_on_a_hexproof_creature_can_be_aimed_at_by_an_opponent() {
    let mut engine = lk::game(
        SEED,
        &[index::SYLVAN_CARYATID, index::HILL_GIANT],
        &[index::GRIZZLY_BEARS],
        &[index::HOLY_STRENGTH],
        &[enchantment_alteration()],
    );
    let caryatid = lk::obj(&engine, P0, index::SYLVAN_CARYATID);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let strength = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, caryatid, &W);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    let targets = cast_at_aura(&mut engine, P1, strength);
    assert!(
        targets.contains(&strength),
        "the opponent may aim at the Aura even though its creature has hexproof: {targets:?}"
    );
    let menu = resolve(&mut engine, Some(their_bears)).expect("two other creatures: it asks");
    assert_eq!(
        sorted(menu),
        sorted(vec![giant, their_bears]),
        "every creature but the hexproof one it was on"
    );
    assert_eq!(kit::attached_to(&engine, strength), Some(their_bears));
    assert_eq!(
        kit::controller_of(&engine, strength),
        P0,
        "it is still seat 0's Aura"
    );
}

/// "… to another permanent": with no other creature on the battlefield there
/// is nowhere to move it. The spell is still cast, still resolves, and the
/// Aura stays on its creature (CR 701.3b). The control is the same board with
/// a second creature, where it moves.
#[test]
fn with_no_other_creature_the_aura_stays_where_it_is() {
    for (other_creature, moves) in [(true, true), (false, false)] {
        let mine: &[CardIndex] = if other_creature {
            &[index::GRIZZLY_BEARS, index::HILL_GIANT]
        } else {
            &[index::GRIZZLY_BEARS]
        };
        let mut engine = lk::game(
            SEED,
            mine,
            &[],
            &[index::HOLY_STRENGTH, enchantment_alteration()],
            &[],
        );
        let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
        let strength = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, bears, &W);

        let targets = cast_at_aura(&mut engine, P0, strength);
        assert!(
            targets.contains(&strength),
            "still a legal target: {targets:?}"
        );
        let giant = on_battlefield(&engine, P0, index::HILL_GIANT);
        resolve(&mut engine, giant);

        if moves {
            assert_eq!(
                kit::attached_to(&engine, strength),
                giant,
                "with a second creature the Aura moves"
            );
        } else {
            assert_eq!(
                kit::attached_to(&engine, strength),
                Some(bears),
                "with no other creature it stays"
            );
            assert!(
                kit::on_the_table(&engine, strength),
                "and is not lost to the graveyard"
            );
        }
        assert!(
            in_graveyard(&engine, P0, enchantment_alteration()).is_some(),
            "the instant resolved either way"
        );
    }
}

/// "… another permanent of that type" is read together with what the Aura can
/// enchant (CR 701.3a): Animate Wall says "Enchant Wall", so on a Wall of Wood
/// it can only go to another Wall -- the Wall of Stone -- and never to the
/// Bears or the Giant standing next to it. With no other Wall it does not
/// move at all (CR 701.3b), though the Bears and the Giant are creatures.
#[test]
fn an_aura_only_moves_to_a_creature_it_could_have_enchanted() {
    for other_wall in [true, false] {
        let theirs: &[CardIndex] = if other_wall {
            &[index::WALL_OF_STONE]
        } else {
            &[]
        };
        let mut engine = lk::game(
            SEED,
            &[index::WALL_OF_WOOD, index::GRIZZLY_BEARS, index::HILL_GIANT],
            theirs,
            &[index::ANIMATE_WALL, enchantment_alteration()],
            &[],
        );
        let wood = lk::obj(&engine, P0, index::WALL_OF_WOOD);
        let animate = kit::cast_aura(&mut engine, P0, index::ANIMATE_WALL, wood, &W);
        let stone = on_battlefield(&engine, P1, index::WALL_OF_STONE);

        cast_at_aura(&mut engine, P0, animate);
        let menu = resolve(&mut engine, stone);

        if other_wall {
            let stone = stone.expect("the other Wall is out");
            assert!(
                menu.as_ref().is_none_or(|menu| *menu == vec![stone]),
                "if it asks, the only host on the menu is the other Wall: {menu:?}"
            );
            assert_eq!(
                kit::attached_to(&engine, animate),
                Some(stone),
                "the Aura went to the Wall of Stone"
            );
        } else {
            assert!(
                menu.as_ref().is_none_or(Vec::is_empty),
                "no creature it could enchant: nothing to choose from: {menu:?}"
            );
            assert_eq!(
                kit::attached_to(&engine, animate),
                Some(wood),
                "it stays on the Wall of Wood: the Bears and the Giant are not Walls"
            );
        }
    }
}
