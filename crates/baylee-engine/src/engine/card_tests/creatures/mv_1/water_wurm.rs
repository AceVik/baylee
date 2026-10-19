//! `cards/creatures/mv_1/water_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn table(mine: &[CardIndex], theirs: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut mine_all = vec![card_index("cfa373c4-6e42-4cf3-94a4-841f2824603e")];
    mine_all.extend_from_slice(mine);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &mine_all)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let wurm = on_battlefield(&e, P0, card_index("cfa373c4-6e42-4cf3-94a4-841f2824603e"))
        .expect("the Wurm is out");
    (e, wurm)
}

/// Water Wurm (cfa373c4-6e42-4cf3-94a4-841f2824603e) — "This creature gets
/// +0/+1 as long as an opponent controls an Island."
#[test]
fn water_wurm_is_one_one_when_the_opponent_has_no_island() {
    let (e, wurm) = table(&[], &[forest(), mountain()]);
    assert_eq!(pt(&e, wurm), (1, 1), "other lands do not count");
}

#[test]
fn water_wurm_gets_a_toughness_while_an_opponent_controls_an_island() {
    let (e, wurm) = table(&[], &[island()]);
    assert_eq!(pt(&e, wurm), (1, 2));
}

#[test]
fn water_wurm_ignores_an_island_of_its_own_controller() {
    let (e, wurm) = table(&[island()], &[]);
    assert_eq!(pt(&e, wurm), (1, 1));
}

#[test]
fn water_wurm_loses_the_toughness_when_the_island_leaves() {
    let stone_rain = card_index("6e880df6-fc00-43d2-a9c8-f575f40b78c6");
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card_index("cfa373c4-6e42-4cf3-94a4-841f2824603e"),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[island()])
        .hand(0, &[stone_rain])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let wurm = on_battlefield(&e, P0, card_index("cfa373c4-6e42-4cf3-94a4-841f2824603e"))
        .expect("the Wurm is out");
    assert_eq!(pt(&e, wurm), (1, 2));
    let isle = on_battlefield(&e, P1, island()).expect("their Island is out");
    tap_all_mana(&mut e, P0);
    cast_from_hand(&mut e, P0, stone_rain);
    let p = e.pending().asked().expect("Stone Rain asks for its target");
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![isle],
            players: vec![],
        },
    )
    .expect("the Island is a legal target");
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P1, island()).is_none(),
        "Stone Rain destroyed it"
    );
    assert_eq!(pt(&e, wurm), (1, 1));
}
