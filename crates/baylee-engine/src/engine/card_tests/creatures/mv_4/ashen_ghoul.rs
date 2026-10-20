//! `cards/creatures/mv_4/ashen_ghoul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ashen_ghoul() -> CardIndex {
    card_index("46fbcbdf-73d0-4fa2-8db7-3b19bfc7ac22")
}

/// Ashen Ghoul (46fbcbdf-73d0-4fa2-8db7-3b19bfc7ac22) — {3}{B} 3/1 Creature — Zombie:
/// - Haste
/// - "{B}: Return Ashen Ghoul from your graveyard to the battlefield. Activate
///   only during your upkeep and only if three or more creature cards are above
///   Ashen Ghoul."
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
                gray_ogre(),
                festering_goblin(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Put Ashen Ghoul into graveyard first (bottom), then two creatures above it.
    let ghoul_id = hand_to_graveyard(&mut engine, P0, ashen_ghoul());
    hand_to_graveyard(&mut engine, P0, llanowar_elves());
    hand_to_graveyard(&mut engine, P0, gray_ogre());

    // Advance to P1's main phase, then to P0's upkeep: with only 2 creatures
    // above Ashen Ghoul, it cannot be activated even with {B} floating.
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
        "only two creature cards are above Ashen Ghoul, so the ability is not legal"
    );

    // Put a third creature card on top of P0's graveyard during P0's main phase.
    walk_to_own_main(&mut engine, P0);
    hand_to_graveyard(&mut engine, P0, festering_goblin());

    // Outside upkeep (in FirstMain), the ability is still not legal.
    assert!(
        !priority_offer(&engine).abilities.contains(&(ghoul_id, 0)),
        "cannot activate outside your upkeep"
    );

    // Advance to P0's next upkeep: now 3 creature cards are above Ashen Ghoul!
    reach_their_main_phase(&mut engine, P1);
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
