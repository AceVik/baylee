//! `cards/instants/mv_1/reaping_the_rewards.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn reaping_the_rewards() -> CardIndex {
    card_index("b4177445-46d8-4427-a345-d786c820c3af")
}

/// p0 has `lands` Plains and Reaping the Rewards in hand; p1 has two Islands
/// and a Counterspell. p0's main phase.
fn reaping_over(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &vec![plains(); lands])
        .battlefield(1, &[island(), island()])
        .hand(0, &[reaping_the_rewards()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Names `land` for the buyback cost, and hands back the lands it was named
/// out of.
#[track_caller]
fn sacrifice(engine: &mut Engine<RegistryLookup>, land: ObjectId) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected the land to sacrifice, got {:?}", engine.pending())
    };
    assert_eq!(player, P0, "the caster pays");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
    options
}

/// Reaping the Rewards (`b4177445-46d8-4427-a345-d786c820c3af`) — {W} Instant:
/// "You gain 2 life."
#[test]
fn reaping_the_rewards_gains_2_life() {
    let mut engine = reaping_over(1);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, reaping_the_rewards());
    answer_buyback(&mut engine, P0, false);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 22);
    assert!(in_graveyard(&engine, P0, reaping_the_rewards()).is_some());
}

/// "Buyback—Sacrifice a land": a land of the caster's pays, and the land that
/// made the spell's own mana is as good as any: being tapped is no bar.
#[test]
fn reaping_the_rewards_with_a_land_sacrificed_comes_back_to_hand() {
    let mut engine = reaping_over(2);
    let lands = all_on_battlefield(&engine, P0, plains());
    let theirs = on_battlefield(&engine, P1, island()).unwrap();

    cast_from_hand(&mut engine, P0, reaping_the_rewards());
    answer_buyback(&mut engine, P0, true);
    assert!(is_tapped(&engine, lands[0]));
    let menu = sacrifice(&mut engine, lands[0]);
    assert!(menu.contains(&lands[1]), "either of the caster's lands");
    assert!(!menu.contains(&theirs), "and no land of anyone else's");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 22);
    assert!(in_graveyard(&engine, P0, plains()).is_some());
    assert_eq!(all_on_battlefield(&engine, P0, plains()), vec![lands[1]]);
    assert_bought_back(&engine, P0, reaping_the_rewards());
}

/// Declined, every land stays and it is a Reaping the Rewards that is spent.
#[test]
fn reaping_the_rewards_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = reaping_over(2);

    cast_from_hand(&mut engine, P0, reaping_the_rewards());
    answer_buyback(&mut engine, P0, false);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 22);
    assert_eq!(all_on_battlefield(&engine, P0, plains()).len(), 2);
    assert!(in_graveyard(&engine, P0, plains()).is_none());
    assert_not_bought_back(&engine, P0, reaping_the_rewards());
}

/// Countered, it does not resolve: land gone, card gone, no life gained.
/// "You gain 2 life" targets nothing, so no target can leave it (CR 608.2b).
#[test]
fn reaping_the_rewards_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = reaping_over(2);
    let land = on_battlefield(&engine, P0, plains()).unwrap();

    cast_from_hand(&mut engine, P0, reaping_the_rewards());
    answer_buyback(&mut engine, P0, true);
    sacrifice(&mut engine, land);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_graveyard(&engine, P0, plains()).is_some());
    assert_not_bought_back(&engine, P0, reaping_the_rewards());
}

/// With no land to give there is nothing to ask.
#[test]
fn reaping_the_rewards_with_no_land_is_not_asked() {
    let mut engine = reaping_over(0);
    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);

    cast_with_floating(&mut engine, P0, reaping_the_rewards());
    assert!(
        on_stack(&engine, reaping_the_rewards()).is_some(),
        "cast with no question, got {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 22);
    assert_not_bought_back(&engine, P0, reaping_the_rewards());
}

/// The costs of CR 601.2h come after the mana of CR 601.2g, so the only land
/// there is may pay the {W} and then be the one sacrificed. The tests above
/// tap everything first; this one casts off an untapped land, as a player
/// does, and the sacrifice is named before the land is tapped.
#[test]
fn the_only_land_pays_the_mana_and_is_then_sacrificed() {
    let mut engine = reaping_over(1);
    let land = on_battlefield(&engine, P0, plains()).unwrap();
    let spell = in_hand(&engine, P0, reaping_the_rewards()).unwrap();

    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    answer_buyback(&mut engine, P0, true);
    assert!(!is_tapped(&engine, land), "nothing has been paid yet");
    sacrifice(&mut engine, land);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(on_stack(&engine, reaping_the_rewards()).is_some());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 22);
    assert!(all_on_battlefield(&engine, P0, plains()).is_empty());
    assert!(in_graveyard(&engine, P0, plains()).is_some());
    assert_bought_back(&engine, P0, reaping_the_rewards());
}
