//! `cards/creatures/mv_2/magus_of_the_unseen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn magus_of_the_unseen() -> CardIndex {
    card_index("d8857977-c8a5-4703-9aaa-331b17c7a713")
}

/// Magus of the Unseen (d8857977-c8a5-4703-9aaa-331b17c7a713) — {1}{U} 1/1 Creature — Human Wizard:
/// "{1}{U}, {T}: Untap target artifact an opponent controls and gain control of it until end of turn. It gains haste until end of turn. When you lose control of the artifact, tap it."
#[test]
fn magus_of_the_unseen_untaps_gains_control_of_and_grants_haste_to_an_opponents_artifact_until_eot()
{
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[magus_of_the_unseen(), sol_ring(), island(), island()])
        .battlefield(1, &[fellwar_stone()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let own_ring = on_battlefield(&engine, P0, sol_ring()).expect("own Sol Ring seated");
    let opp_stone = on_battlefield(&engine, P1, fellwar_stone()).expect("opponent Fellwar Stone");

    // Tap P1's Fellwar Stone before activating Magus of the Unseen so we can verify UntapTarget.
    engine.dev_state_mut(P0).unwrap().set_tapped(opp_stone, true);
    assert!(entered_tapped(&engine, opp_stone));

    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, magus_of_the_unseen(), 0);
    let menu = aim_at(&mut engine, P0, opp_stone);
    assert!(
        menu.contains(&opp_stone) && !menu.contains(&own_ring),
        "only artifacts an opponent controls may be targeted, not your own"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !entered_tapped(&engine, opp_stone),
        "Magus of the Unseen untaps the targeted artifact"
    );
    assert_eq!(
        engine.state().object(opp_stone).unwrap().controller,
        P0,
        "P0 gains control of the artifact until end of turn"
    );
    assert!(
        keywords(&engine, opp_stone).contains(KeywordSet::HASTE),
        "the artifact gains haste until end of turn"
    );

    // Advance past end of turn into P1's upkeep: control and haste both expire in cleanup.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().object(opp_stone).unwrap().controller,
        P1,
        "control returns to P1 at end of turn"
    );
    assert!(
        !keywords(&engine, opp_stone).contains(KeywordSet::HASTE),
        "haste expires at end of turn"
    );
}
