//! `cards/sorceries/mv_2/fanning_the_flames.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fanning_the_flames() -> CardIndex {
    card_index("1d5f0a1c-88e0-4603-97af-8ed47766812e")
}

/// p0 has `lands` Mountains and Fanning the Flames in hand; p1 has a Gray
/// Ogre, two Islands and a Counterspell. p0's main phase.
fn flames_over(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &vec![mountain(); lands])
        .battlefield(1, &[gray_ogre(), island(), island()])
        .hand(0, &[fanning_the_flames()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Aims the spell at p1's face.
#[track_caller]
fn aim_at_p1(engine: &mut Engine<RegistryLookup>) {
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
}

/// Fanning the Flames (`1d5f0a1c-88e0-4603-97af-8ed47766812e`) — {X}{R}{R} Sorcery:
/// "This spell deals X damage to any target."
#[test]
fn fanning_the_flames_deals_x_damage_to_any_target() {
    let mut engine = flames_over(5);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, fanning_the_flames());
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    aim_at_p1(&mut engine);
    pass_until(&mut engine, stack_is_empty);
    assert_not_bought_back(&engine, P0, fanning_the_flames());

    assert_eq!(engine.state().players[1].life, 17);
}

/// "Buyback {3}": X is asked first, then the buyback, and the total is X and
/// {R}{R} and {3}.
#[test]
fn fanning_the_flames_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = flames_over(8);

    cast_from_hand(&mut engine, P0, fanning_the_flames());
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    answer_buyback(&mut engine, P0, true);
    aim_at_p1(&mut engine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "3 and {{R}}{{R}} and {{3}} are the eight floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
    assert_bought_back(&engine, P0, fanning_the_flames());
}

/// Declined, it costs three less and is a Fanning the Flames that is spent.
#[test]
fn fanning_the_flames_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = flames_over(8);

    cast_from_hand(&mut engine, P0, fanning_the_flames());
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    answer_buyback(&mut engine, P0, false);
    aim_at_p1(&mut engine);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
    assert_not_bought_back(&engine, P0, fanning_the_flames());
}

/// An X that spends all the mana leaves none for the {3}, and with no land
/// left to tap nothing is asked.
#[test]
fn fanning_the_flames_with_an_x_that_spends_everything_is_not_asked() {
    let mut engine = flames_over(8);

    cast_from_hand(&mut engine, P0, fanning_the_flames());
    engine.apply(P0, PlayerAction::ChooseNumber(6)).unwrap();

    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "got {:?}",
        engine.pending()
    );
}

/// Countered, it does not resolve: buyback paid, card gone, no damage.
#[test]
fn fanning_the_flames_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = flames_over(8);

    cast_from_hand(&mut engine, P0, fanning_the_flames());
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    answer_buyback(&mut engine, P0, true);
    aim_at_p1(&mut engine);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, fanning_the_flames());
}

/// With its one target gone it does not resolve either.
#[test]
fn fanning_the_flames_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = flames_over(8);
    let ogre = on_battlefield(&engine, P1, gray_ogre()).unwrap();

    cast_from_hand(&mut engine, P0, fanning_the_flames());
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, ogre);
    bury(&mut engine, &[ogre]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, fanning_the_flames());
}
