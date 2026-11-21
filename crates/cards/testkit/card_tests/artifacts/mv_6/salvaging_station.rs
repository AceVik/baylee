//! `cards/artifacts/mv_6/salvaging_station.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn salvaging_station() -> CardIndex {
    card_index("3501ed98-473c-4a17-8b5f-1e0c00ee75da")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

/// Salvaging Station (3501ed98-473c-4a17-8b5f-1e0c00ee75da) — {6} Artifact:
/// "{T}: Return target noncreature artifact card with mana value 1 or less
///  from your graveyard to the battlefield."
/// "Whenever a creature dies, you may untap this artifact."
///
/// Target filter admits MV 0 and MV 1 noncreature artifacts in YOUR graveyard
/// (Lotus Petal, Sol Ring) and rejects artifact creatures (Ornithopter), MV 2
/// artifacts (Millstone), and opponent's MV 1 artifacts; creature deaths on
/// either side offer optional untap (accept and decline both verified).
#[test]
fn salvaging_station_returns_mv1_noncreature_artifacts_and_untaps_on_creature_death() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                salvaging_station(),
                llanowar_elves(),
                mountain(),
                mountain(),
            ],
        )
        .hand(
            0,
            &[
                lightning_bolt(),
                lightning_bolt(),
                sol_ring(),
                lotus_petal(),
                ornithopter(),
                millstone(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let station = on_battlefield(&engine, p0, salvaging_station()).unwrap();
    let my_ring = hand_to_graveyard(&mut engine, p0, sol_ring());
    let my_petal = hand_to_graveyard(&mut engine, p0, lotus_petal());
    let my_thopter = hand_to_graveyard(&mut engine, p0, ornithopter());
    let my_stone = hand_to_graveyard(&mut engine, p0, millstone());
    let their_ring = hand_to_graveyard(&mut engine, p1, sol_ring());
    engine.refresh_offer();

    // 1. Activate {T} -> return MV 1 Sol Ring; verify filter boundaries.
    activate(&mut engine, p0, salvaging_station(), 0);
    let menu = aim_at(&mut engine, p0, my_ring);
    assert!(
        menu.contains(&my_ring) && menu.contains(&my_petal),
        "MV 0 and MV 1 noncreature artifacts in own graveyard are legal: {menu:?}"
    );
    assert!(
        !menu.contains(&my_thopter) && !menu.contains(&my_stone) && !menu.contains(&their_ring),
        "artifact creature, MV 2 artifact, and opponent's graveyard excluded: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, station));
    assert_eq!(
        engine.state().object(my_ring).unwrap().zone,
        Zone::Battlefield
    );

    // 2. Kill opponent's creature -> creature dies trigger -> decline untap -> stays tapped.
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let mtns = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| id == mtns[0]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, their_elf);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, station),
        "declining keeps Station tapped"
    );

    // 3. Kill own creature -> creature dies trigger -> accept untap -> untaps and returns Petal!
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == mtns[1]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, my_elf);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, station),
        "accepted untap on creature death"
    );

    activate(&mut engine, p0, salvaging_station(), 0);
    aim_at(&mut engine, p0, my_petal);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(my_petal).unwrap().zone,
        Zone::Battlefield,
        "MV 0 Lotus Petal returned to the battlefield"
    );
}
