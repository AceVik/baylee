//! `cards/sorceries/mv_2/pegasus_stampede.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn pegasus_stampede() -> CardIndex {
    card_index("16b4a48a-fec2-433c-948d-e0b16c961172")
}

/// p0 has `lands` Plains and Pegasus Stampede in hand; p1 has two Islands and
/// a Counterspell. p0's main phase.
fn a_stampede_over(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &vec![plains(); lands])
        .battlefield(1, &[island(), island()])
        .hand(0, &[pegasus_stampede()])
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

/// Pegasus Stampede (`16b4a48a-fec2-433c-948d-e0b16c961172`) — {1}{W} Sorcery:
/// "Create a 1/1 white Pegasus creature token with flying."
#[test]
fn pegasus_stampede_creates_1_1_white_pegasus_token_with_flying() {
    let mut engine = a_stampede_over(2);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, pegasus_stampede());
    answer_buyback(&mut engine, P0, false);
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, P0);
    assert_eq!(tokens.len(), 1, "a Pegasus token was created");
    assert_eq!(pt(&engine, tokens[0]), (1, 1));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::FLYING));
}

/// "Buyback—Sacrifice a land": a land of the caster's pays, and the land that
/// made the spell's own mana is as good as any: being tapped is no bar.
#[test]
fn pegasus_stampede_with_a_land_sacrificed_comes_back_to_hand() {
    let mut engine = a_stampede_over(2);
    let lands = all_on_battlefield(&engine, P0, plains());
    let theirs = on_battlefield(&engine, P1, island()).unwrap();

    cast_from_hand(&mut engine, P0, pegasus_stampede());
    answer_buyback(&mut engine, P0, true);
    assert!(is_tapped(&engine, lands[0]));
    let menu = sacrifice(&mut engine, lands[0]);
    assert!(menu.contains(&lands[1]), "either of the caster's lands");
    assert!(!menu.contains(&theirs), "and no land of anyone else's");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(tokens_of(&engine, P0).len(), 1);
    assert!(in_graveyard(&engine, P0, plains()).is_some());
    assert_eq!(all_on_battlefield(&engine, P0, plains()), vec![lands[1]]);
    assert_bought_back(&engine, P0, pegasus_stampede());
}

/// Declined, every land stays and it is a Pegasus Stampede that is spent.
#[test]
fn pegasus_stampede_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_stampede_over(2);

    cast_from_hand(&mut engine, P0, pegasus_stampede());
    answer_buyback(&mut engine, P0, false);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(tokens_of(&engine, P0).len(), 1);
    assert_eq!(all_on_battlefield(&engine, P0, plains()).len(), 2);
    assert!(in_graveyard(&engine, P0, plains()).is_none());
    assert_not_bought_back(&engine, P0, pegasus_stampede());
}

/// Countered, it does not resolve: land gone, card gone, no Pegasus. "Create
/// a token" targets nothing, so no target can leave it (CR 608.2b).
#[test]
fn pegasus_stampede_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_stampede_over(2);
    let land = on_battlefield(&engine, P0, plains()).unwrap();

    cast_from_hand(&mut engine, P0, pegasus_stampede());
    answer_buyback(&mut engine, P0, true);
    sacrifice(&mut engine, land);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(tokens_of(&engine, P0).is_empty());
    assert!(in_graveyard(&engine, P0, plains()).is_some());
    assert_not_bought_back(&engine, P0, pegasus_stampede());
}

/// With no land to give there is nothing to ask.
#[test]
fn pegasus_stampede_with_no_land_is_not_asked() {
    let mut engine = a_stampede_over(0);
    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);

    cast_with_floating(&mut engine, P0, pegasus_stampede());
    assert!(
        on_stack(&engine, pegasus_stampede()).is_some(),
        "cast with no question, got {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(tokens_of(&engine, P0).len(), 1);
    assert_not_bought_back(&engine, P0, pegasus_stampede());
}
