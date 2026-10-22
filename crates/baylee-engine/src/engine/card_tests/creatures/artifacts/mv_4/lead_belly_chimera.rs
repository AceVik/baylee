//! `cards/creatures/artifacts/mv_4/lead_belly_chimera.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn lead_belly_chimera() -> CardIndex {
    card_index("e8fb2323-3eed-45a6-bfc4-44292c8b3697")
}

fn brass_talon_chimera() -> CardIndex {
    card_index("2e114d15-4356-4cef-8f89-260ab4c8d4b9")
}

/// `Lead-Belly Chimera` ({4}, 2/2 Artifact Creature — Chimera,
/// `Coverage::Implemented`) has trample and sacrifices itself to put a +2/+2
/// counter on target Chimera creature and grant it trample indefinitely.
#[test]
fn lead_belly_chimera_sacrifices_to_put_p2p2_counter_and_trample_on_target_chimera() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                brass_talon_chimera(),
            ],
        )
        .hand(0, &[lead_belly_chimera()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, lead_belly_chimera());
    pass_until(&mut engine, stack_is_empty);
    let lead_belly = on_battlefield(&engine, p0, lead_belly_chimera()).unwrap();
    assert!(keywords(&engine, lead_belly).contains(KeywordSet::TRAMPLE));

    let target_chimera = on_battlefield(&engine, p0, brass_talon_chimera()).unwrap();
    activate(&mut engine, p0, lead_belly_chimera(), 0);
    aim_at(&mut engine, p0, target_chimera);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(lead_belly).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        counters_on(
            &engine,
            target_chimera,
            CounterKind::Plus {
                power: 2,
                toughness: 2
            }
        ),
        1
    );
    assert_eq!(pt(&engine, target_chimera), (4, 4));
    assert!(keywords(&engine, target_chimera).contains(KeywordSet::TRAMPLE));
}
