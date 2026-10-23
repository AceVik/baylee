//! `cards/creatures/mv_4/flailing_drake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flailing_drake() -> CardIndex {
    card_index("93b6a170-a816-4556-aa27-a729231715e7")
}

/// Flailing Drake (`93b6a170-a816-4556-aa27-a729231715e7`) — {3}{G} Creature — Drake:
/// "Flying. Whenever this creature blocks or becomes blocked by a creature, that creature gets +1/+1 until end of turn."
#[test]
fn flailing_drake_pumps_creature_it_blocks_by_plus_one_plus_one() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::GRIZZLY_BEARS])
        .battlefield(1, &[flailing_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let drake = on_battlefield(&engine, P1, flailing_drake()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(drake, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 3));
}

#[test]
fn flailing_drake_pumps_only_its_blocker_and_only_until_end_of_turn() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[flailing_drake(), index::GRIZZLY_BEARS],
        &[index::WALL_OF_AIR],
        &[],
        &[],
    );
    let drake = lk::obj(&e, P0, flailing_drake());
    let bears = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    let wall = lk::obj(&e, P1, index::WALL_OF_AIR);
    let menu = lk::attack(&mut e, &[drake]);
    assert!(lk::may_block(&menu, wall, drake));
    lk::block(&mut e, &[(wall, drake)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, wall), (2, 6));
    assert_eq!(pt(&e, bears), (2, 2));
    assert_eq!(pt(&e, drake), (2, 3));
    assert!(keywords(&e, drake).contains(KeywordSet::FLYING));
    reach_their_main_phase(&mut e, P1);
    assert_eq!(pt(&e, wall), (1, 5));
}
