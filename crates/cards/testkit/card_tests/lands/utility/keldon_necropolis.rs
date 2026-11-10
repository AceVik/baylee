//! `cards/lands/utility/keldon_necropolis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Keldon Necropolis: "{4}{R}, {T}, Sacrifice a creature: Keldon Necropolis
/// deals 2 damage to any target." A Mountain pays the {R}.
#[test]
fn keldon_necropolis_sacrifices_a_creature_to_deal_two_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let necropolis = card_index("ea4d6fcd-21e0-4e9f-b406-a89042998d98");
    let mut engine = Duel::new(2103, forest())
        .battlefield(
            0,
            &[
                necropolis,
                mountain(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let n = on_battlefield(&engine, p0, necropolis).expect("necropolis");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf");
    tap_mana_where(&mut engine, p0, |id| id != n && id != elf);
    let before = engine.state().players[1].life;

    activate(&mut engine, p0, necropolis, 1);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(player_options.contains(&p1), "any target includes players");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    unf_aim_and_pay(&mut engine, p0, None, Some(elf));

    assert_eq!(engine.state().players[1].life, before - 2);
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(is_tapped(&engine, n));
}
