//! `cards/instants/mv_2/constant_mists.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn constant_mists() -> CardIndex {
    card_index("c850a29e-dc40-4ab6-89f1-d501a1a350d1")
}

/// p0 has two Forests and Grizzly Bears, with Constant Mists in hand; p1 has
/// two Islands and a Counterspell. p0's main phase.
fn mists_over_two_forests() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), index::GRIZZLY_BEARS])
        .battlefield(1, &[island(), island()])
        .hand(0, &[constant_mists()])
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

/// Sends `attacker` at p1 and walks the game to p0's second main phase.
#[track_caller]
fn attack_p1(engine: &mut Engine<RegistryLookup>, attacker: ObjectId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

/// Constant Mists (`c850a29e-dc40-4ab6-89f1-d501a1a350d1`) — {1}{G} Instant:
/// "Prevent all combat damage that would be dealt this turn."
#[test]
fn constant_mists_prevents_all_combat_damage() {
    let mut engine = mists_over_two_forests();

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, constant_mists());
    answer_buyback(&mut engine, P0, false);
    pass_until(&mut engine, stack_is_empty);
    assert_not_bought_back(&engine, P0, constant_mists());
    assert_eq!(
        all_on_battlefield(&engine, P0, forest()).len(),
        2,
        "declined, no land is given"
    );

    attack_p1(&mut engine, bears);

    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn fog_does_not_stop_noncombat_damage_and_ends_with_the_turn() {
    let mut e = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[constant_mists(), lightning_bolt()],
        &[],
    );
    let bear = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    lk::float(&mut e, P0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, P0, constant_mists());
    lk::settle(&mut e);
    // Pay the green spell before adding red: its generic cost may otherwise spend red.
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, P0, lightning_bolt(), &[], &[P1]);
    assert_eq!(lk::life(&e, P1), 17, "noncombat damage is not prevented");
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    lk::attackers_offered(&mut e);
    lk::declare(&mut e, &[bear]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(
        lk::life(&e, P1),
        15,
        "next turn combat damage is not prevented"
    );
}

/// "Buyback—Sacrifice a land": a land of the caster's pays, and the land that
/// made the spell's own mana is as good as any: being tapped is no bar.
#[test]
fn constant_mists_with_a_land_sacrificed_comes_back_to_hand() {
    let mut engine = mists_over_two_forests();
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let lands = all_on_battlefield(&engine, P0, forest());
    let theirs = on_battlefield(&engine, P1, island()).unwrap();

    cast_from_hand(&mut engine, P0, constant_mists());
    answer_buyback(&mut engine, P0, true);
    assert!(is_tapped(&engine, lands[0]));
    let menu = sacrifice(&mut engine, lands[0]);
    assert!(menu.contains(&lands[1]), "either of the caster's lands");
    assert!(!menu.contains(&theirs), "and no land of anyone else's");
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, forest()).is_some());
    assert_eq!(all_on_battlefield(&engine, P0, forest()), vec![lands[1]]);
    assert_bought_back(&engine, P0, constant_mists());

    attack_p1(&mut engine, bears);
    assert_eq!(engine.state().players[1].life, 20);
}

/// Countered, it does not resolve: land gone, card gone, and combat deals its
/// damage. "Prevent all combat damage" targets nothing, so no target can leave
/// it (CR 608.2b).
#[test]
fn constant_mists_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = mists_over_two_forests();
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let land = on_battlefield(&engine, P0, forest()).unwrap();

    cast_from_hand(&mut engine, P0, constant_mists());
    answer_buyback(&mut engine, P0, true);
    sacrifice(&mut engine, land);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(in_graveyard(&engine, P0, forest()).is_some());
    assert_not_bought_back(&engine, P0, constant_mists());

    attack_p1(&mut engine, bears);
    assert_eq!(engine.state().players[1].life, 18);
}

/// With no land to give there is nothing to ask.
#[test]
fn constant_mists_with_no_land_is_not_asked() {
    let mut e = lk::game(SEED, &[], &[], &[constant_mists()], &[]);
    lk::float(&mut e, P0, &[(ManaColor::Green, 2)]);

    cast_with_floating(&mut e, P0, constant_mists());

    assert!(
        on_stack(&e, constant_mists()).is_some(),
        "cast with no question, got {:?}",
        e.pending()
    );
}
