//! `cards/creatures/artifacts/mv_4/iron_heart_chimera.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn iron_heart_chimera() -> CardIndex {
    card_index("fe7c70dd-76ff-4958-9973-6452c7f071ed")
}

fn brass_talon_chimera() -> CardIndex {
    card_index("2e114d15-4356-4cef-8f89-260ab4c8d4b9")
}

/// `Iron-Heart Chimera` ({4}, 2/2 Artifact Creature — Chimera,
/// `Coverage::Implemented`) has vigilance and sacrifices itself to put a +2/+2
/// counter on target Chimera creature and grant it vigilance indefinitely.
#[test]
fn iron_heart_chimera_sacrifices_to_put_p2p2_counter_and_vigilance_on_target_chimera() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                brass_talon_chimera(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[brass_talon_chimera()])
        .hand(0, &[iron_heart_chimera()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, iron_heart_chimera());
    pass_until(&mut engine, stack_is_empty);
    let iron_heart = on_battlefield(&engine, p0, iron_heart_chimera()).unwrap();
    assert!(keywords(&engine, iron_heart).contains(KeywordSet::VIGILANCE));

    let target_chimera = on_battlefield(&engine, p0, brass_talon_chimera()).unwrap();
    let opp_chimera = on_battlefield(&engine, p1, brass_talon_chimera()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    activate(&mut engine, p0, iron_heart_chimera(), 0);
    let menu = aim_at(&mut engine, p0, target_chimera);
    assert!(
        menu.contains(&opp_chimera),
        "any Chimera creature, the opponent's too"
    );
    assert!(
        !menu.contains(&bears),
        "a creature that is no Chimera is not offered"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(iron_heart).unwrap().zone,
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
    assert!(keywords(&engine, target_chimera).contains(KeywordSet::VIGILANCE));

    assert_eq!(
        pt(&engine, opp_chimera),
        (2, 2),
        "the other Chimera was not touched"
    );
    assert!(!keywords(&engine, opp_chimera).contains(KeywordSet::VIGILANCE));
    assert_eq!(pt(&engine, bears), (2, 2));

    // "This effect lasts indefinitely": the counter and the vigilance outlive the turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, target_chimera), (4, 4));
    assert!(keywords(&engine, target_chimera).contains(KeywordSet::VIGILANCE));
}
