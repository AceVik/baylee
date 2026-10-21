//! `cards/creatures/mv_3/flow_of_maggots.rs`, played.
//!
//! Oracle:
//! - "Cumulative upkeep {1}"
//! - "This creature can't be blocked by non-Wall creatures."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flow_of_maggots() -> CardIndex {
    card_index("2d6a387f-7ad4-419e-951a-9dcd4c9ac823")
}

#[test]
fn flow_of_maggots_cant_be_blocked_by_non_wall_and_has_cumulative_upkeep() {
    let mut engine = lk::game_with(
        SEED,
        &[flow_of_maggots()],
        &[index::GRIZZLY_BEARS, index::WALL_OF_WOOD],
        &[],
        &[],
        |e| {
            lk::float(e, P0, &[(ManaColor::Colorless, 1)]);
            lk::yes_no(e, true);
        },
    );
    let maggots = lk::obj(&engine, P0, flow_of_maggots());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let wall = lk::obj(&engine, P1, index::WALL_OF_WOOD);
    assert_eq!(
        engine
            .state()
            .object(maggots)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    let offer = lk::attack(&mut engine, &[maggots]);
    assert!(
        !lk::may_block(&offer, bears, maggots),
        "non-Wall Grizzly Bears cannot block Flow of Maggots"
    );
    assert!(
        lk::may_block(&offer, wall, maggots),
        "Wall of Wood can block Flow of Maggots"
    );
}

/// "Cumulative upkeep {1}" (CR 702.24a): the first upkeep is paid in the
/// fixture; the next puts a second age counter on, asks {1} for each of the
/// two, and declined, the Maggots are sacrificed.
#[test]
fn flow_of_maggots_cumulative_upkeep_grows_and_is_sacrificed_when_unpaid() {
    let mut engine = lk::game_with(SEED, &[flow_of_maggots()], &[], &[], &[], |e| {
        lk::float(e, P0, &[(ManaColor::Colorless, 1)]);
        lk::yes_no(e, true);
    });
    let maggots = lk::obj(&engine, P0, flow_of_maggots());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, P0, "asked of the Maggots' controller");
    assert_eq!(engine.state().turn.active, P0, "at its own upkeep");
    assert_eq!(
        engine
            .state()
            .object(maggots)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2,
        "a second age counter went on first"
    );
    assert_eq!(
        prompt,
        YesNoPrompt::PayTax { mana: 2 },
        "{{1}} for each of the two age counters"
    );
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::settle(&mut engine);
    assert!(
        in_graveyard(&engine, P0, flow_of_maggots()).is_some(),
        "unpaid, Flow of Maggots is sacrificed"
    );
}
