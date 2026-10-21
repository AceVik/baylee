//! `cards/creatures/mv_4/ashen_ghoul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ashen_ghoul() -> CardIndex {
    card_index("46fbcbdf-73d0-4fa2-8db7-3b19bfc7ac22")
}

/// Puts {B} in P0's pool and republishes the offer, so that a check of the
/// timing gate is not answered by an empty pool instead: an ability from the
/// graveyard is offered only when its cost can be paid.
fn float_black(engine: &mut Engine<RegistryLookup>) {
    engine.state.players[0].mana_pool.add(ManaColor::Black, 1);
    engine.refresh_offer();
}

/// Ashen Ghoul (46fbcbdf-73d0-4fa2-8db7-3b19bfc7ac22) — {3}{B} 3/1 Creature — Zombie:
/// - Haste
/// - "{B}: Return Ashen Ghoul from your graveyard to the battlefield. Activate
///   only during your upkeep and only if three or more creature cards are above
///   Ashen Ghoul."
///
/// With {B} available every time it is asked: not with two creature cards and
/// a land above it in P0's upkeep, not with three in P0's main phase, not in
/// P1's upkeep, and then in P0's upkeep it returns.
#[test]
fn ashen_ghoul_has_haste_and_returns_from_graveyard_during_your_upkeep_with_three_creatures_above_it()
 {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(
            0,
            &[
                ashen_ghoul(),
                llanowar_elves(),
                forest(),
                gray_ogre(),
                festering_goblin(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Ashen Ghoul at the bottom; above it two creature cards and a land.
    let ghoul_id = hand_to_graveyard(&mut engine, P0, ashen_ghoul());
    hand_to_graveyard(&mut engine, P0, llanowar_elves());
    hand_to_graveyard(&mut engine, P0, forest());
    hand_to_graveyard(&mut engine, P0, gray_ogre());

    // P0's upkeep, {B} floating: three cards above it, two of them creatures.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    let swamp_id = on_battlefield(&engine, P0, swamp()).expect("Swamp seated");
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: swamp_id })
        .expect("tap Swamp for {B}");
    assert!(
        !priority_offer(&engine).abilities.contains(&(ghoul_id, 0)),
        "the Forest above it is not a creature card, so only two count"
    );

    // P0's main phase: a third creature card on top, {B} in the pool.
    walk_to_own_main(&mut engine, P0);
    hand_to_graveyard(&mut engine, P0, festering_goblin());
    float_black(&mut engine);
    assert!(
        !priority_offer(&engine).abilities.contains(&(ghoul_id, 0)),
        "cannot activate outside your upkeep"
    );

    // P1's upkeep, P0 holding priority with {B}: an upkeep, but not P0's.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    float_black(&mut engine);
    assert!(
        !priority_offer(&engine).abilities.contains(&(ghoul_id, 0)),
        "an opponent's upkeep is not your upkeep"
    );

    // P0's next upkeep: three creature cards above it.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: swamp_id })
        .expect("tap Swamp for {B}");
    assert!(
        priority_offer(&engine).abilities.contains(&(ghoul_id, 0)),
        "three creature cards are above Ashen Ghoul during P0's upkeep"
    );

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: ghoul_id,
                ability_index: 0,
            },
        )
        .expect("activate Ashen Ghoul from graveyard");
    pass_until(&mut engine, stack_is_empty);

    let returned = on_battlefield(&engine, P0, ashen_ghoul()).expect("Ashen Ghoul returned");
    assert_eq!(pt(&engine, returned), (3, 1));
    assert!(keywords(&engine, returned).contains(KeywordSet::HASTE));
}
