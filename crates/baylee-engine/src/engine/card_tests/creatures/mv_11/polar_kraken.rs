//! `cards/creatures/mv_11/polar_kraken.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);

fn polar_kraken() -> CardIndex {
    card_index("d0ff30c8-ddb7-439d-b14d-c83fe3c8da87")
}

/// Polar Kraken (d0ff30c8-ddb7-439d-b14d-c83fe3c8da87) — {8}{U}{U}{U} 11/11 Creature — Kraken:
/// - "Trample"
/// - "This creature enters tapped."
/// - "Cumulative upkeep—Sacrifice a land." (Coverage::Partial — sacrifices one land each upkeep.)
#[test]
fn polar_kraken_enters_tapped_with_trample_and_sacrifices_a_land_on_upkeep_or_is_sacrificed() {
    let mut lands = vec![island(); 11];
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &lands)
        .hand(0, &[polar_kraken()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, polar_kraken());
    pass_until(&mut engine, stack_is_empty);

    let kraken = on_battlefield(&engine, P0, polar_kraken()).unwrap();
    assert!(
        is_tapped(&engine, kraken),
        "Polar Kraken enters the battlefield tapped"
    );
    assert!(keywords(&engine, kraken).contains(KeywordSet::TRAMPLE));

    // Upkeep 1 (turn 3): 1 age counter placed; P0 accepts and sacrifices an Island.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::CostSacrifice,
                    ..
                }
            )
    });
    let one_island = on_battlefield(&engine, P0, island()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![one_island],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().object(one_island).unwrap().zone,
        Zone::Graveyard,
        "paying the upkeep cost sacrifices the chosen land"
    );
    assert!(
        on_battlefield(&engine, P0, polar_kraken()).is_some(),
        "Polar Kraken remains on the battlefield when a land is sacrificed"
    );

    // Upkeep 2 (turn 5): 2nd age counter placed; P0 declines (chooses 0 lands) and sacrifices Polar Kraken.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5
            && matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::CostSacrifice,
                    ..
                }
            )
    });
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, polar_kraken()).is_none()
            && in_graveyard(&engine, P0, polar_kraken()).is_some(),
        "Polar Kraken is sacrificed when the upkeep land sacrifice is declined"
    );
    lands.clear();
}
