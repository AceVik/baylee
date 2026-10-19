//! `cards/artifacts/mv_2/tower_of_coireall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::BlockOption;

fn tower_of_coireall() -> CardIndex {
    card_index("f9afad36-933f-4d18-acb7-32e5cb5c8161")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn wall_of_wood() -> CardIndex {
    card_index("aa5e6894-8c97-46f8-a0fd-07c6db51e9a7")
}

/// p0 declares `attackers` at p1 and the engine stops on p1's block question,
/// whose offer is returned.
#[track_caller]
fn attack_and_see_blocks(
    engine: &mut Engine<RegistryLookup>,
    attackers: &[ObjectId],
) -> Vec<BlockOption> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&a| (a, Defender::Player(p1)))
                    .collect(),
            },
        )
        .expect("the attackers were on the offer");
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the block question")
    };
    blockers
}

fn can_block(offer: &[BlockOption], blocker: ObjectId, attacker: ObjectId) -> bool {
    offer
        .iter()
        .any(|b| b.blocker == blocker && b.attackers.contains(&attacker))
}

struct Table {
    engine: Engine<RegistryLookup>,
    bears: ObjectId,
    lions: ObjectId,
    wall: ObjectId,
    giant: ObjectId,
}

/// p0: Tower, Bears, Lions. p1: Wall of Wood and Hill Giant. p0's main phase.
fn a_table() -> Table {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tower_of_coireall(), grizzly_bears(), savannah_lions()])
        .battlefield(1, &[wall_of_wood(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("Bears");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("Lions");
    let wall = on_battlefield(&engine, p1, wall_of_wood()).expect("Wall");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("Giant");
    Table {
        engine,
        bears,
        lions,
        wall,
        giant,
    }
}

/// Tower of Coireall (f9afad36-933f-4d18-acb7-32e5cb5c8161) — "{T}: Target
/// creature can't be blocked by Walls this turn."
///
/// Aimed at the Bears: the Wall is not offered as their blocker, the Giant
/// still is; the Lions, not the target, may still be blocked by the Wall; and
/// naming the Wall as the Bears' blocker is refused.
#[test]
fn tower_of_coireall_keeps_walls_off_the_targeted_attacker_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let Table {
        mut engine,
        bears,
        lions,
        wall,
        giant,
    } = a_table();
    let tower = on_battlefield(&engine, p0, tower_of_coireall()).expect("Tower");
    activate(&mut engine, p0, tower_of_coireall(), 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, tower), "{{T}} was the price");

    let offer = attack_and_see_blocks(&mut engine, &[bears, lions]);
    assert!(
        !can_block(&offer, wall, bears),
        "a Wall cannot block the Bears"
    );
    assert!(can_block(&offer, giant, bears), "a non-Wall still can");
    assert!(
        can_block(&offer, wall, lions),
        "the Lions were not the target: the Wall may block them"
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(wall, bears)],
                },
            )
            .is_err(),
        "the engine refuses the Wall on the Bears"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, bears)],
            },
        )
        .expect("the Giant may block the Bears");
}

/// Control: with no activation the Wall blocks the Bears like anything else.
#[test]
fn tower_of_coireall_unused_leaves_walls_free_to_block() {
    let Table {
        mut engine,
        bears,
        wall,
        ..
    } = a_table();
    let offer = attack_and_see_blocks(&mut engine, &[bears]);
    assert!(can_block(&offer, wall, bears));
}

/// "This turn": on the next turn the Bears are blockable by the Wall again.
#[test]
fn tower_of_coireall_wears_off_at_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let Table {
        mut engine,
        bears,
        wall,
        ..
    } = a_table();
    activate(&mut engine, p0, tower_of_coireall(), 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    let offer = attack_and_see_blocks(&mut engine, &[bears]);
    assert!(!can_block(&offer, wall, bears));
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    reach_their_main_phase(&mut engine, p1);
    let offer = attack_and_see_blocks(&mut engine, &[bears]);
    assert!(
        can_block(&offer, wall, bears),
        "a new turn: the Wall may block the Bears"
    );
}
