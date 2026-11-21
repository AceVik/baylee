//! `cards/instants/mv_7/soul_spike.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "be979bfc-998d-43d5-a7a4-44548d9a14c4"
fn soul_spike() -> CardIndex {
    card_index("be979bfc-998d-43d5-a7a4-44548d9a14c4")
}

/// Soul Spike ({5}{B}{B}): alternative cost requires exiling TWO other black
/// cards from your hand (not castable with only one black + one red card);
/// deals 4 damage to any target and gains you 4 life.
#[test]
fn soul_spike_casts_via_exiling_two_black_cards_and_drains_four() {
    let not_enough = lk::game(
        SEED,
        &[],
        &[],
        &[soul_spike(), index::DARK_RITUAL, index::LIGHTNING_BOLT],
        &[],
    );
    assert!(
        !lk::castable(&not_enough, P0, soul_spike()),
        "1 other black card + 1 red card is not enough for alternative cost"
    );

    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[
            soul_spike(),
            index::DARK_RITUAL,
            index::TERROR,
            index::LIGHTNING_BOLT,
        ],
        &[],
    );
    let ritual = in_hand(&engine, P0, index::DARK_RITUAL).unwrap();
    let terror = in_hand(&engine, P0, index::TERROR).unwrap();
    let bolt = in_hand(&engine, P0, index::LIGHTNING_BOLT).unwrap();

    cast_with_floating(&mut engine, P0, soul_spike());
    for pitch in [ritual, terror] {
        loop {
            match engine.pending().clone() {
                Pending::ChooseCastMode { options, .. } => {
                    let slot = options
                        .iter()
                        .position(|o| matches!(o.kind, CastModeKind::Alternative(0)))
                        .expect("alternative cost offered");
                    engine.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
                }
                Pending::ChooseTargets { .. } => {
                    lk::aim(&mut engine, &[], &[P1]);
                }
                Pending::ChooseCards { options, .. } => {
                    assert!(
                        !options.contains(&bolt),
                        "red card cannot be exiled for black pitch cost: {options:?}"
                    );
                    engine
                        .apply(
                            P0,
                            PlayerAction::ChooseObjects {
                                objects: vec![pitch],
                            },
                        )
                        .unwrap();
                    break;
                }
                other => panic!("unexpected pending during Soul Spike cast: {other:?}"),
            }
        }
    }
    lk::settle(&mut engine);

    assert_eq!(engine.state().object(ritual).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(terror).unwrap().zone, Zone::Exile);
    assert!(in_hand(&engine, P0, index::LIGHTNING_BOLT).is_some());
    assert_eq!(lk::life(&engine, P1), 16, "dealt 4 damage to P1");
    assert_eq!(lk::life(&engine, P0), 24, "gained 4 life");
}

/// Soul Spike cast for its printed mana cost ({5}{B}{B}) kills a 4/4 creature
/// and gains 4 life.
#[test]
fn soul_spike_cast_for_mana_kills_creature_and_gains_four_life() {
    let mut engine = lk::game(SEED, &[], &[index::SERRA_ANGEL], &[soul_spike()], &[]);
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);

    lk::float(&mut engine, P0, &[(ManaColor::Black, 7)]);
    cast_with_floating(&mut engine, P0, soul_spike());
    lk::aim(&mut engine, &[angel], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 0);
    assert!(in_graveyard(&engine, P1, index::SERRA_ANGEL).is_some());
    assert_eq!(lk::life(&engine, P0), 24);
}
