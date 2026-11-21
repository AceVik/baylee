//! `cards/creatures/mv_2/stoic_champion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn stoic_champion() -> CardIndex {
    card_index("1203a80f-11ac-46ed-aa2a-5a6782b6303d")
}

fn secluded_steppe() -> CardIndex {
    card_index("8dec6fcf-1254-4b1b-ba23-7a3e492a7241")
}

fn putrid_imp() -> CardIndex {
    card_index("7192ff07-9a84-49ad-a40c-ce255ce21544")
}

/// Stoic Champion gets +2/+2 until end of turn whenever any player cycles a
/// card (stacking across multiple cycles in one turn), ignores plain discards,
/// and reverts to 2/2 at cleanup.
#[test]
fn stoic_champion_pumps_on_either_players_cycle_ignores_discard_and_expires() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), stoic_champion(), putrid_imp()])
        .battlefield(1, &[plains()])
        .hand(0, &[secluded_steppe(), swamp()])
        .hand(1, &[secluded_steppe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let champ = on_battlefield(&engine, p0, stoic_champion()).expect("Stoic Champion seated");
    assert_eq!(pt(&engine, champ), (2, 2));

    // Plain discard via Putrid Imp does not pump Stoic Champion.
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
    assert_eq!(pt(&engine, champ), (2, 2));

    // P0 cycles Secluded Steppe -> 4/4.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, secluded_steppe(), 1);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, champ), (4, 4));

    // P1 cycles in response on P0's turn -> stacks to 6/6.
    pass_once(&mut engine);
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, secluded_steppe(), 1);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, champ), (6, 6));

    // Cleanup restores base 2/2.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, champ), (2, 2));
}
