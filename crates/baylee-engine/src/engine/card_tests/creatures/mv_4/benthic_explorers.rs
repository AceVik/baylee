//! Benthic Explorers — `{3}{U}` 2/4 Creature — Merfolk Scout (Masters Edition III)
//!
//! `{T}`, Untap a tapped land an opponent controls: Add one mana of any type that land could
//! produce.
//!
//! The ability is off the card: nothing untaps another player's land as a cost, and nothing reads
//! what a land could produce. A bare `{T}`-for-any-color ability would be free mana without the
//! cost or the limit, so this test pins that the creature has no mana ability yet.

#[allow(clippy::wildcard_imports)]
use super::*;

fn benthic_explorers() -> CardIndex {
    card_index("bc0518fc-d34b-474a-92c5-2cb013828396")
}

#[test]
fn benthic_explorers_has_no_mana_ability_it_could_play_wrongly() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 4])
        .battlefield(1, &[island()])
        .hand(0, &[benthic_explorers()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, benthic_explorers());
    pass_until(&mut engine, stack_is_empty);
    let explorers =
        on_battlefield(&engine, p0, benthic_explorers()).expect("Benthic Explorers enters");
    assert_eq!(pt(&engine, explorers), (2, 4));

    // Advance to P0's next main phase so summoning sickness wears off.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let pool_before = engine.state().players[0].mana_pool.total();

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: explorers,
                    ability_index: 0,
                },
            )
            .is_err(),
        "no mana ability: {{T}} alone would not be the printed cost"
    );
    assert!(!is_tapped(&engine, explorers));
    assert_eq!(engine.state().players[0].mana_pool.total(), pool_before);
}
