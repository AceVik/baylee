//! `cards/creatures/mv_4/benthic_explorers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn benthic_explorers() -> CardIndex {
    card_index("bc0518fc-d34b-474a-92c5-2cb013828396")
}

/// `Benthic Explorers` ({3}{U}, 2/4 Creature — Merfolk Scout,
/// `Coverage::Partial`) taps to add one mana of any color.
#[test]
fn benthic_explorers_taps_for_one_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 4])
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
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: explorers,
                ability_index: 0,
            },
        )
        .unwrap();
    if let Pending::ChooseColor { .. } = engine.pending() {
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
            .unwrap();
    }

    assert!(is_tapped(&engine, explorers));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}
