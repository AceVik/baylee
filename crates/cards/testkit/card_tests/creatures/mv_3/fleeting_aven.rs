//! `cards/creatures/mv_3/fleeting_aven.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn fleeting_aven() -> CardIndex {
    card_index("7215b1d9-eb8b-41bc-a7e2-1930d9cd5365")
}

fn lonely_sandbar() -> CardIndex {
    card_index("765863c8-1be0-4bb1-9e9c-db7701cffde3")
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

fn putrid_imp() -> CardIndex {
    card_index("7192ff07-9a84-49ad-a40c-ce255ce21544")
}

/// Fleeting Aven has flying, ignores plain discards, and returns to its OWNER's
/// hand whenever any player cycles a card (even while controlled by an opponent).
#[test]
fn fleeting_aven_has_flying_bounces_to_owners_hand_on_any_cycle_not_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                fleeting_aven(),
                putrid_imp(),
            ],
        )
        .battlefield(
            1,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[lonely_sandbar(), swamp()])
        .hand(1, &[control_magic(), lonely_sandbar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let aven = on_battlefield(&engine, p0, fleeting_aven()).expect("Fleeting Aven seated");
    let bear1 = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Bear");
    // Flying in combat: ground Bear cannot block attacking Fleeting Aven.
    let blockers = attack_and_collect_blocks(&mut engine, aven, p1);
    assert!(
        !may_block(&blockers, bear1, aven),
        "ground creature cannot block flying Fleeting Aven"
    );
    block(&mut engine, &[]);
    through_combat(&mut engine);
    assert_eq!(life(&engine, p1), 18);

    // Plain discard via Putrid Imp does not bounce Fleeting Aven.
    let swamp_card = in_hand(&engine, p0, swamp()).expect("Swamp in hand");
    activate(&mut engine, p0, putrid_imp(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swamp_card],
            },
        )
        .expect("discard Swamp");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(zone_of(&engine, aven), Zone::Battlefield);

    // P0 cycles Lonely Sandbar -> Fleeting Aven returns to P0's hand.
    let islands0 = all_on_battlefield(&engine, p0, island());
    tap_mana_where(&mut engine, p0, |id| id == islands0[0]);
    activate(&mut engine, p0, lonely_sandbar(), 1);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(in_hand(&engine, p0, fleeting_aven()).is_some());

    // Recast Fleeting Aven; on P1's turn P1 steals it with Control Magic and
    // cycles Lonely Sandbar -> returns to its OWNER's (P0's) hand, not P1's.
    tap_mana_where(&mut engine, p0, |id| islands0[1..4].contains(&id));
    cast_with_floating(&mut engine, p0, fleeting_aven());
    pass_until(&mut engine, stack_is_empty);
    let aven2 = on_battlefield(&engine, p0, fleeting_aven()).expect("recast Aven");

    reach_their_main_phase(&mut engine, p1);
    let islands1 = all_on_battlefield(&engine, p1, island());
    tap_mana_where(&mut engine, p1, |id| islands1[..4].contains(&id));
    cast_with_floating(&mut engine, p1, control_magic());
    aim(&mut engine, &[aven2], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(aven2).unwrap().controller, p1);

    tap_mana_where(&mut engine, p1, |id| id == islands1[4]);
    activate(&mut engine, p1, lonely_sandbar(), 1);
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert!(
        in_hand(&engine, p0, fleeting_aven()).is_some(),
        "returned to owner P0's hand"
    );
    assert!(in_hand(&engine, p1, fleeting_aven()).is_none());
}
