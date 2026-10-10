//! `cards/creatures/artifacts/mv_2/battering_ram.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

use crate::turn::{Phase, Step};

/// Battering Ram: "At the beginning of combat on your turn, this creature
/// gains banding until end of combat." and "Whenever this creature becomes
/// blocked by a Wall, destroy that Wall at end of combat."
fn battering_ram() -> CardIndex {
    card_index("e7b91fba-8d96-4040-95e8-f0023b65c497")
}

/// Turn one with the Ram in play on p0's side and `theirs` on p1's, the
/// attack declared with the Ram alone, stopped at the block question.
fn ram_attacks(theirs: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[battering_ram()])
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ram = on_battlefield(&engine, p0, battering_ram()).expect("the Ram is out");
    let blockers: Vec<ObjectId> = theirs
        .iter()
        .map(|c| on_battlefield(&engine, p1, *c).expect("their creature is out"))
        .collect();
    let options = attack_and_collect_blocks(&mut engine, ram, p1);
    for b in &blockers {
        assert!(
            options
                .iter()
                .any(|o| o.blocker == *b && o.attackers.contains(&ram)),
            "{b:?} may block the Ram: {options:?}"
        );
    }
    (engine, ram, blockers)
}

fn block_ram(engine: &mut Engine<RegistryLookup>, ram: ObjectId, blockers: &[ObjectId]) {
    engine
        .apply(
            PlayerId::new(1),
            PlayerAction::DeclareBlockers {
                blockers: blockers.iter().map(|b| (*b, ram)).collect(),
            },
        )
        .expect("the blocks are legal");
}

fn alive(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .contains(&id)
}

/// The Ram has banding from the beginning of combat on its controller's
/// turn, through the attack, and not after combat ends.
#[test]
fn battering_ram_gains_banding_for_your_combat_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, ram, _) = ram_attacks(&[grizzly_bears()]);
    assert!(
        keywords(&engine, ram).contains(KeywordSet::BANDING),
        "banding at the declare-blockers question"
    );
    block_ram(&mut engine, ram, &[]);
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatEnd);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::End
    });
    assert!(
        !keywords(&engine, ram).contains(KeywordSet::BANDING),
        "banding ended with combat"
    );
    // Not on the opponent's turn either.
    reach_their_main_phase(&mut engine, p1);
    assert!(!keywords(&engine, ram).contains(KeywordSet::BANDING));
}

/// A Wall that blocks the Ram is destroyed at end of combat, not before.
#[test]
fn battering_ram_destroys_a_blocking_wall_at_end_of_combat() {
    let (mut engine, ram, walls) = ram_attacks(&[wall_of_stone()]);
    let wall = walls[0];
    block_ram(&mut engine, ram, &walls);
    // The trigger goes on the stack at the block; the delayed destroy waits.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatDamage && stack_is_empty(e)
    });
    assert!(alive(&engine, wall), "alive through the damage step");
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatEnd);
    pass_until(&mut engine, |e| {
        e.state().turn.step != Step::CombatEnd && stack_is_empty(e)
    });
    assert!(!alive(&engine, wall), "destroyed at end of combat");
    assert!(
        in_graveyard(&engine, PlayerId::new(1), wall_of_stone()).is_some(),
        "in its owner's graveyard"
    );
    assert!(alive(&engine, ram), "the Ram is untouched by a 0/8");
}

/// A creature that is not a Wall blocks the Ram and nothing is queued.
#[test]
fn battering_ram_leaves_a_non_wall_blocker_alone() {
    let (mut engine, ram, others) = ram_attacks(&[grizzly_bears()]);
    let bears = others[0];
    block_ram(&mut engine, ram, &others);
    pass_until(&mut engine, |e| {
        e.state().turn.active == PlayerId::new(0) && e.state().turn.step == Step::End
    });
    // The 2/2 Bears and the 1/1 Ram trade damage: the Ram dies, the Bears live.
    assert!(alive(&engine, bears), "no trigger, the Bears survive");
    assert!(
        !alive(&engine, ram),
        "the 1/1 Ram died to the Bears' damage"
    );
}

/// Two Walls block the Ram: each is its own trigger, both are destroyed.
#[test]
fn battering_ram_destroys_every_wall_that_blocks_it() {
    let (mut engine, ram, walls) = ram_attacks(&[wall_of_stone(), wall_of_wood()]);
    block_ram(&mut engine, ram, &walls);
    // A banded Ram divides its own damage between its blockers (CR 702.22).
    for _ in 0..200 {
        match engine.pending().clone() {
            Pending::ChooseNumber { player, max, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseNumber(max))
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                if engine.state().turn.step == Step::CombatEnd {
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
    pass_until(&mut engine, |e| {
        e.state().turn.step != Step::CombatEnd && stack_is_empty(e)
    });
    for w in &walls {
        assert!(!alive(&engine, *w), "{w:?} destroyed");
    }
}

/// One-sided: when the Ram blocks, nothing is destroyed, and the Ram has no
/// banding on the opponent's turn.
#[test]
fn battering_ram_blocking_triggers_nothing_and_has_no_banding() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[battering_ram(), wall_of_wood()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    let ram = on_battlefield(&engine, p0, battering_ram()).expect("the Ram");
    let wall = on_battlefield(&engine, p0, wall_of_wood()).expect("the Wall");
    reach_their_main_phase(&mut engine, p1);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("the Bears");
    let options = attack_and_collect_blocks(&mut engine, bears, p0);
    assert!(options.iter().any(|o| o.blocker == ram));
    assert!(!keywords(&engine, ram).contains(KeywordSet::BANDING));
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ram, bears)],
            },
        )
        .expect("legal");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain || e.state().turn.step == Step::End
    });
    assert!(
        alive(&engine, wall),
        "the friendly Wall, not blocking, is not destroyed"
    );
}
