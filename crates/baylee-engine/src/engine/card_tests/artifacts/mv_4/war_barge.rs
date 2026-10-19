//! `cards/artifacts/mv_4/war_barge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::BlockOption;

fn war_barge() -> CardIndex {
    card_index("fd57331f-1cfa-4f88-a925-fc171738c3c0")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn shatter() -> CardIndex {
    card_index("84d32e68-72b9-432d-a023-7bc6a26b8ad0")
}

fn draconian_cylix() -> CardIndex {
    card_index("0880b36a-6141-4955-b532-cf88daca0869")
}

fn islandwalking(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    keywords(engine, id).contains(KeywordSet::ISLANDWALK)
}

/// p0 declares the attackers at p1 and the engine stops on p1's block
/// question, whose offer is returned.
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

/// p0: Barge, Bears, `forests` Forests (and `extra`); p1: `theirs`. p0's main.
fn a_barge_board(
    forests: usize,
    extra: &[CardIndex],
    theirs: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut mine = vec![war_barge(), grizzly_bears()];
    mine.extend_from_slice(extra);
    mine.extend(std::iter::repeat_n(forest(), forests));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(1, theirs)
        .hand(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let barge = on_battlefield(&engine, p0, war_barge()).expect("the Barge");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears");
    (engine, barge, bears)
}

/// Pays {3} with Forests `which` and resolves the Barge's ability on `target`.
#[track_caller]
fn barge_at(engine: &mut Engine<RegistryLookup>, target: ObjectId, which: &[ObjectId]) {
    let p0 = PlayerId::new(0);
    assert_eq!(tap_mana_where(engine, p0, |id| which.contains(&id)), 3);
    activate(engine, p0, war_barge(), 0);
    aim_at(engine, p0, target);
    pass_until(engine, stack_is_empty);
}

/// War Barge (fd57331f-1cfa-4f88-a925-fc171738c3c0) — "{3}: Target creature
/// gains islandwalk until end of turn. When this artifact leaves the
/// battlefield this turn, destroy that creature. A creature destroyed this way
/// can't be regenerated."
///
/// With an Island across the table, the Bears can't be blocked; the Hill Giant
/// is a legal blocker for them only when the opponent has no Island.
#[test]
fn war_barge_grants_islandwalk_and_the_defender_with_an_island_cannot_block() {
    for (theirs_land, blockable) in [(island(), false), (forest(), true)] {
        let (mut engine, _barge, bears) = a_barge_board(3, &[], &[hill_giant(), theirs_land]);
        let giant = on_battlefield(&engine, PlayerId::new(1), hill_giant()).expect("Giant");
        assert!(!islandwalking(&engine, bears), "not before the ability");
        let forests = all_on_battlefield(&engine, PlayerId::new(0), forest());
        barge_at(&mut engine, bears, &forests);
        assert!(islandwalking(&engine, bears), "islandwalk is granted");

        let offer = attack_and_see_blocks(&mut engine, &[bears]);
        assert_eq!(
            offer
                .iter()
                .any(|b| b.blocker == giant && b.attackers.contains(&bears)),
            blockable,
            "the Giant may block the Bears only without an Island opposite (Island: {})",
            !blockable
        );
    }
}

/// "Until end of turn": the keyword is gone when the next turn comes.
#[test]
fn war_barge_islandwalk_ends_with_the_turn() {
    let (mut engine, _barge, bears) = a_barge_board(3, &[], &[]);
    let forests = all_on_battlefield(&engine, PlayerId::new(0), forest());
    barge_at(&mut engine, bears, &forests);
    assert!(islandwalking(&engine, bears));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(!islandwalking(&engine, bears), "the grant lapsed");
}

/// The Barge leaves the battlefield this turn (Shatter): the creature it
/// helped is destroyed, a trigger of its own after the Shatter resolves.
#[test]
fn war_barge_leaving_destroys_the_creature_it_helped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                war_barge(),
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
                forest(),
                mountain(),
            ],
        )
        .hand(0, &[shatter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let barge = on_battlefield(&engine, p0, war_barge()).expect("the Barge");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears");
    let forests = all_on_battlefield(&engine, p0, forest());
    barge_at(&mut engine, bears, &forests[..3]);

    let mountain_id = on_battlefield(&engine, p0, mountain()).expect("Mountain");
    let spare_forest = forests[3];
    tap_mana_where(&mut engine, p0, |id| {
        id == mountain_id || id == spare_forest
    });
    cast_with_floating(&mut engine, p0, shatter());
    aim_at(&mut engine, p0, barge);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, war_barge()).is_none(),
        "Shattered"
    );
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_none(),
        "and the Bears with it"
    );
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, war_barge()).is_some());
}

/// "This turn": the Barge leaving on the opponent's turn does nothing to the
/// Bears.
#[test]
fn war_barge_leaving_after_the_turn_destroys_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, barge, bears) = a_barge_board(3, &[], &[]);
    let forests = all_on_battlefield(&engine, p0, forest());
    barge_at(&mut engine, bears, &forests);
    reach_their_main_phase(&mut engine, p1);
    kill(&mut engine, barge);
    assert!(on_battlefield(&engine, p0, war_barge()).is_none());
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "the Barge's promise was for the turn it was activated"
    );
}

/// "A creature destroyed this way can't be regenerated": the Bears hold a
/// Cylix regeneration shield and the Barge goes anyway; they are destroyed.
#[test]
fn war_barge_destruction_ignores_a_regeneration_shield() {
    let p0 = PlayerId::new(0);
    let (mut engine, barge, bears) = a_barge_board(5, &[draconian_cylix()], &[]);
    let forests = all_on_battlefield(&engine, p0, forest());
    barge_at(&mut engine, bears, &forests[..3]);

    assert_eq!(
        tap_mana_where(&mut engine, p0, |id| forests[3..].contains(&id)),
        2
    );
    activate(&mut engine, p0, draconian_cylix(), 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).unwrap().regeneration_shields,
        1,
        "the shield is standing"
    );

    kill(&mut engine, barge);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_none(),
        "destroyed despite the shield"
    );
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
}
