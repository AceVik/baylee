//! Extraplanar Lens — `{3}` Artifact (MRD).
//! Oracle: "Imprint — When this artifact enters, you may exile target land you control. Whenever a land with the same name as the exiled card is tapped for mana, its controller adds one mana of any type that land produced."

use super::*;

fn extraplanar_lens() -> CardIndex {
    card_index("e48f9fc7-72cf-4c3a-b3b3-d237edc01310")
}

#[test]
fn extraplanar_lens_imprints_a_land_and_doubles_mana_from_lands_with_same_name() {
    let p0 = PlayerId::new(0);
    let lens = extraplanar_lens();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), island()])
        .hand(0, &[lens])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = all_on_battlefield(&engine, p0, forest());
    let isl = on_battlefield(&engine, p0, island()).unwrap();
    tap_mana_where(&mut engine, p0, |id| forests[..3].contains(&id));
    cast_with_floating(&mut engine, p0, lens);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, forests[0]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(all_on_battlefield(&engine, p0, forest()).len(), 3);

    // Tapping the remaining untapped Forest triggers Extraplanar Lens for an extra {G}.
    tap_mana_where(&mut engine, p0, |id| id == forests[3]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2
    );

    // Tapping an Island does not trigger Extraplanar Lens.
    tap_mana_where(&mut engine, p0, |id| id == isl);
    assert!(stack_is_empty(&engine));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
}
