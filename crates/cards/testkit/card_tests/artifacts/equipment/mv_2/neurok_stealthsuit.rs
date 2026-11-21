//! `cards/artifacts/equipment/mv_2/neurok_stealthsuit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn neurok_stealthsuit() -> CardIndex {
    card_index("cee06ea6-8bad-4858-b78e-04879a825bbe")
}

/// Neurok Stealthsuit (cee06ea6-8bad-4858-b78e-04879a825bbe) — {2} Equipment:
/// "Equipped creature has shroud."
/// "{U}{U}: Attach this Equipment to target creature you control."
/// "Equip {1}"
///
/// Equip {1} grants Shroud so neither your own Giant Growth nor the opponent's
/// Lightning Bolt can target the equipped creature.
#[test]
fn neurok_stealthsuit_grants_shroud_against_both_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut r = rig_vs(
        neurok_stealthsuit(),
        jasmine_boreal(),
        2,
        &[giant_growth()],
        &[llanowar_elves(), mountain()],
        &[lightning_bolt()],
    );
    r.equip_keeping(1, 1);
    assert!(keywords(&r.engine, r.host).contains(KeywordSet::SHROUD));
    assert!(!keywords(&r.engine, r.bystander).contains(KeywordSet::SHROUD));

    // Own Giant Growth cannot target the shrouded host (only bystander / theirs).
    cast_with_floating(&mut r.engine, p0, giant_growth());
    let menu = aim_at(&mut r.engine, p0, r.bystander);
    assert!(
        !menu.contains(&r.host),
        "Shroud stops your own spells from targeting the equipped creature: {menu:?}"
    );
    pass_until(&mut r.engine, stack_is_empty);

    // Opponent's Lightning Bolt cannot target the shrouded host either.
    reach_their_main_phase(&mut r.engine, p1);
    cast_from_hand(&mut r.engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = r.engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", r.engine.pending())
    };
    assert!(
        !options.contains(&r.host) && options.contains(&r.bystander),
        "Shroud stops opponent's Bolt from targeting the equipped creature: {options:?}"
    );
}

/// "{U}{U}: Attach this Equipment to target creature you control" works at
/// instant speed on the opponent's turn (when Equip {1} is illegal) and moves
/// Shroud to the new host.
#[test]
fn neurok_stealthsuit_uu_attaches_at_instant_speed_on_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                neurok_stealthsuit(),
                jasmine_boreal(),
                goblin_piker(),
                forest(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let suit = on_battlefield(&engine, p0, neurok_stealthsuit()).unwrap();
    let jasmine = on_battlefield(&engine, p0, jasmine_boreal()).unwrap();
    let piker = on_battlefield(&engine, p0, goblin_piker()).unwrap();
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    let my_forest = on_battlefield(&engine, p0, forest()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == my_forest);
    activate(&mut engine, p0, neurok_stealthsuit(), 0);
    aim_at(&mut engine, p0, jasmine);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, jasmine).contains(KeywordSet::SHROUD));

    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);

    assert!(
        !ability_offered(&engine, suit, 0),
        "Equip {{1}} is sorcery-speed only"
    );
    assert!(
        priority_offer(&engine).abilities.contains(&(suit, 2)),
        "{{U}}{{U}} attach is instant-speed on opponent's turn"
    );

    activate(&mut engine, p0, neurok_stealthsuit(), 2);
    let menu = aim_at(&mut engine, p0, piker);
    assert!(menu.contains(&piker) && !menu.contains(&jasmine) && !menu.contains(&their_elf));
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, piker).contains(KeywordSet::SHROUD));
    assert!(!keywords(&engine, jasmine).contains(KeywordSet::SHROUD));
}
