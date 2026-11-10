//! Altar of Shadows — `{7}` Artifact (MRD).
//! Oracle: "At the beginning of your first main phase, add {B} for each charge counter on this artifact. {7}, {T}: Destroy target creature. Then put a charge counter on this artifact."

use super::*;

fn altar_of_shadows() -> CardIndex {
    card_index("4b34a324-d939-453a-97e3-a21c584a0c74")
}

#[test]
fn altar_of_shadows_destroys_creature_adds_charge_counter_and_produces_black_mana_at_first_main() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let altar = altar_of_shadows();
    let elf = llanowar_elves();
    let mut board = vec![swamp(); 7];
    board.push(altar);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &board)
        .battlefield(1, &[elf])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);

    let altar_id = on_battlefield(&engine, p0, altar).unwrap();
    let their_elf = on_battlefield(&engine, p1, elf).unwrap();
    tap_all_mana_but(&mut engine, p0, None);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: altar_id,
                ability_index: 1,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, their_elf);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, elf).is_none());
    assert_eq!(
        engine
            .state()
            .object(altar_id)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );

    // Advance to p0's next first main phase -> Altar of Shadows adds {B}.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
}
