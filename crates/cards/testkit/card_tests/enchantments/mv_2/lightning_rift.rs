//! `cards/enchantments/mv_2/lightning_rift.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn lightning_rift() -> CardIndex {
    card_index("ac3fb7ef-709c-40be-a75b-1b438708388d")
}

fn forgotten_cave() -> CardIndex {
    card_index("394c6de5-7957-4a0b-a6b9-ee0c707cd022")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn putrid_imp() -> CardIndex {
    card_index("7192ff07-9a84-49ad-a40c-ce255ce21544")
}

/// Lightning Rift triggers when any player cycles a card: paying `{1}` deals 2
/// damage to any target (player or creature), declining deals 0 damage, and
/// plain non-cycling discard does not trigger it.
#[test]
fn lightning_rift_pays_or_declines_on_any_players_cycle_not_plain_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                lightning_rift(),
                putrid_imp(),
            ],
        )
        .battlefield(1, &[mountain(), grizzly_bears()])
        .hand(0, &[forgotten_cave(), forgotten_cave(), swamp()])
        .hand(1, &[forgotten_cave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Bear seated");

    // Plain discard via Putrid Imp does not trigger Lightning Rift.
    let swamp_in_hand = in_hand(&engine, p0, swamp()).expect("Swamp in hand");
    activate(&mut engine, p0, putrid_imp(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swamp_in_hand],
            },
        )
        .expect("discard Swamp");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(life(&engine, p1), 20);

    // 1. P0 cycles Forgotten Cave, targets P1, and pays {1} -> 2 damage.
    let reds = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| id == reds[0] || id == reds[1]);
    activate(&mut engine, p0, forgotten_cave(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[], &[p1]);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, true);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(life(&engine, p1), 18);

    // 2. P0 cycles a second Forgotten Cave, targets P1, declines paying {1} -> 0 damage.
    tap_mana_where(&mut engine, p0, |id| id == reds[2] || id == reds[3]);
    activate(&mut engine, p0, forgotten_cave(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[], &[p1]);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(life(&engine, p1), 18, "declining {{1}} deals no damage");

    // 3. Opponent P1 cycles on their turn -> P0's Rift triggers, P0 pays {1} to kill P1's Bear.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, forgotten_cave(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[bear], &[]);
    // P0 floats {1} from the 5th Mountain (kept untapped) before Rift resolves.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_mana_where(&mut engine, p0, |id| id == reds[4]);
    assert_eq!(pool_total(&engine, p0), 1);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, true);
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(pool_total(&engine, p0), 0, "paid {{1}} into Lightning Rift");
    assert_eq!(
        zone_of(&engine, bear),
        Zone::Graveyard,
        "2 damage killed opponent's 2/2 Bear"
    );
}
