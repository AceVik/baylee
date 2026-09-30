//! `Trigger::BlocksOrBecomesBlockedBy` and `Effect::AtEndOfCombat`: "whenever
//! this creature blocks or becomes blocked by a non-Wall creature, destroy
//! that creature at end of combat" (Cockatrice, Thicket Basilisk), played by
//! a creature nobody printed.
//!
//! Three things are under test, and each has a board that tells it from its
//! wrong twin. *That creature* is the other one of the pair, whichever side
//! of the block the source is on (CR 509.3b, 509.3d) — reading the event's
//! own object would destroy the source when it blocks. *At end of combat* is
//! as the end of combat step begins (CR 511.2) — not at combat damage and
//! not at the end step. And the delayed trigger is its own ability once
//! created (CR 603.7e): the source dying in combat does not take it away.

use super::*;
use baylee_cards_dsl::{Filter, TargetSpec, Trigger};
use baylee_core::generated::subtypes;
use baylee_core::ids::Defender;

const BASILISK: u32 = 7200;
const OGRE: u32 = 7201;
const GIANT: u32 = 7202;
const WALL: u32 = 7203;

static NON_WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

static BASILISK_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::BlocksOrBecomesBlockedBy(&NON_WALL),
    &[Effect::AtEndOfCombat {
        about: TargetSpec::EventObject,
        effects: &[Effect::destroy(TargetSpec::EventObject)],
    }]
)];

fn cards() -> Vec<&'static CardDef> {
    vec![
        // 2/4: survives a 3/5's damage, dies to a 5/5's.
        card(
            BASILISK,
            creature_face("Basilisk", "{1}", 2, 4),
            KeywordSet::HASTE,
            BASILISK_ABILITIES,
        ),
        // 3/5: survives the Basilisk's damage and deals it too little.
        card(
            OGRE,
            creature_face("Ogre", "{1}", 3, 5),
            KeywordSet::HASTE,
            &[],
        ),
        // 5/5: survives the Basilisk's damage and kills it.
        card(
            GIANT,
            creature_face("Giant", "{1}", 5, 5),
            KeywordSet::HASTE,
            &[],
        ),
        card(
            WALL,
            FaceDef {
                subtypes: &[subtypes::creature::WALL],
                ..creature_face("Wall", "{1}", 0, 4)
            },
            KeywordSet::EMPTY,
            &[],
        ),
    ]
}

/// Walks to `seat`'s declaration of attackers, declares `attackers` against
/// the other seat, and has the other seat declare `blocks`.
#[track_caller]
fn combat(
    engine: &mut Bench,
    seat: PlayerId,
    attackers: &[ObjectId],
    blocks: &[(ObjectId, ObjectId)],
) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    let defender = if seat == me() { them() } else { me() };
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(defender)))
                    .collect(),
            },
        )
        .expect("the attack is legal");
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == defender),
    );
    engine
        .apply(
            defender,
            PlayerAction::DeclareBlockers {
                blockers: blocks.to_vec(),
            },
        )
        .expect("the blocks are legal");
}

/// Walks until the active player holds priority in `step` on an empty stack.
#[track_caller]
fn to_step(engine: &mut Bench, step: crate::turn::Step) {
    let seat = engine.state().turn.active;
    walk_until(engine, |e| holds_priority_in(e, seat, step));
}

fn on_battlefield(engine: &Bench, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
}

/// The Basilisk blocks an Ogre, and both live through combat damage. At
/// end of combat the Ogre — the other creature — is destroyed and the
/// Basilisk is not, which is the difference between the pair's other
/// creature and the event's own object (the blocker). Combat damage has
/// already been dealt and the Ogre is still there, so the destruction is
/// not a damage effect, and it happens before the combat phase ends.
#[test]
fn the_creature_it_blocks_is_destroyed_at_end_of_combat() {
    let mut engine = bench(
        7200,
        cards(),
        [Seat::with(&[OGRE]), Seat::with(&[BASILISK])],
    );
    let ogre = the(&engine, ZoneLocation::Battlefield, OGRE);
    let basilisk = the(&engine, ZoneLocation::Battlefield, BASILISK);

    combat(&mut engine, me(), &[ogre], &[(basilisk, ogre)]);
    to_step(&mut engine, crate::turn::Step::CombatDamage);
    assert_eq!(
        engine.state().object(ogre).map(|o| o.damage),
        Some(2),
        "combat damage is dealt"
    );
    assert!(
        on_battlefield(&engine, ogre),
        "and the Ogre lives through it"
    );

    to_step(&mut engine, crate::turn::Step::CombatEnd);
    assert_eq!(engine.state().turn.phase, Phase::Combat);
    assert!(
        !on_battlefield(&engine, ogre),
        "destroyed as the end of combat step began"
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), OGRE),
        vec![ogre]
    );
    assert!(on_battlefield(&engine, basilisk), "and the Basilisk stays");
}

/// The Basilisk attacks and an Ogre and a Wall block it: the Ogre is a
/// non-Wall creature and is destroyed at end of combat, the Wall is not.
/// Blocked by two Ogres, it triggers once for each (CR 509.3d) and both
/// are destroyed.
#[test]
fn each_non_wall_creature_blocking_it_is_destroyed_and_a_wall_is_not() {
    for two_ogres in [false, true] {
        let second = if two_ogres { OGRE } else { WALL };
        let mut engine = bench(
            7201,
            cards(),
            [Seat::with(&[BASILISK]), Seat::with(&[OGRE, second])],
        );
        let basilisk = the(&engine, ZoneLocation::Battlefield, BASILISK);
        let blockers: Vec<ObjectId> = [OGRE, WALL]
            .iter()
            .flat_map(|&c| objects(&engine, ZoneLocation::Battlefield, c))
            .collect();
        assert_eq!(blockers.len(), 2);

        let blocks: Vec<_> = blockers.iter().map(|b| (*b, basilisk)).collect();
        combat(&mut engine, me(), &[basilisk], &blocks);
        to_step(&mut engine, crate::turn::Step::CombatEnd);

        let ogres_left = objects(&engine, ZoneLocation::Battlefield, OGRE).len();
        let walls_left = objects(&engine, ZoneLocation::Battlefield, WALL).len();
        assert_eq!(ogres_left, 0, "two Ogres: {two_ogres}");
        assert_eq!(
            walls_left,
            usize::from(!two_ogres),
            "the Wall is spared: {two_ogres}"
        );
    }
}

/// The Basilisk blocks a Giant and dies to its combat damage. The delayed
/// trigger its ability created is an ability of its own (CR 603.7e), and
/// the Giant is destroyed at end of combat all the same.
#[test]
fn the_other_creature_is_destroyed_after_the_source_has_died() {
    let mut engine = bench(
        7202,
        cards(),
        [Seat::with(&[GIANT]), Seat::with(&[BASILISK])],
    );
    let giant = the(&engine, ZoneLocation::Battlefield, GIANT);
    let basilisk = the(&engine, ZoneLocation::Battlefield, BASILISK);

    combat(&mut engine, me(), &[giant], &[(basilisk, giant)]);
    to_step(&mut engine, crate::turn::Step::CombatDamage);
    assert!(!on_battlefield(&engine, basilisk), "the Basilisk died");
    assert!(on_battlefield(&engine, giant));

    to_step(&mut engine, crate::turn::Step::CombatEnd);
    assert!(!on_battlefield(&engine, giant), "and the Giant followed it");
}
