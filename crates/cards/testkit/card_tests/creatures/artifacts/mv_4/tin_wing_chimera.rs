//! `cards/creatures/artifacts/mv_4/tin_wing_chimera.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn tin_wing_chimera() -> CardIndex {
    card_index("0d5fe27f-db51-4323-aa74-b6fdb4309425")
}

fn lead_belly_chimera() -> CardIndex {
    card_index("e8fb2323-3eed-45a6-bfc4-44292c8b3697")
}

/// `Tin-Wing Chimera` ({4}, 2/2 Artifact Creature — Chimera,
/// `Coverage::Implemented`) has flying and sacrifices itself to put a +2/+2
/// counter on target Chimera creature and grant it flying indefinitely.
#[test]
fn tin_wing_chimera_sacrifices_to_put_p2p2_counter_and_flying_on_chimera() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                lead_belly_chimera(),
                GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[lead_belly_chimera()])
        .hand(0, &[tin_wing_chimera()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let recipient = on_battlefield(&engine, p0, lead_belly_chimera()).unwrap();
    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let opposing = on_battlefield(&engine, p1, lead_belly_chimera()).unwrap();

    cast_from_hand(&mut engine, p0, tin_wing_chimera());
    pass_until(&mut engine, stack_is_empty);
    let tin = on_battlefield(&engine, p0, tin_wing_chimera()).unwrap();
    assert!(keywords(&engine, tin).contains(KeywordSet::FLYING));

    activate(&mut engine, p0, tin_wing_chimera(), 0);
    let menu = aim_at(&mut engine, p0, recipient);
    assert!(menu.contains(&recipient));
    assert!(
        menu.contains(&opposing),
        "\"target Chimera creature\" is any Chimera, an opponent's included"
    );
    assert!(
        !menu.contains(&bear),
        "a creature that is not a Chimera is no target"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, tin_wing_chimera()).is_some());
    assert_eq!(pt(&engine, recipient), (4, 4));
    assert!(keywords(&engine, recipient).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, bear), (2, 2));
    assert_eq!(pt(&engine, opposing), (2, 2));
    // "Indefinitely": the counter and flying outlast the turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, recipient), (4, 4));
    assert!(keywords(&engine, recipient).contains(KeywordSet::FLYING));
}
