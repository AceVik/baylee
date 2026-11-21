//! `cards/creatures/mv_6/crosis_the_purger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn crosis_the_purger() -> CardIndex {
    card_index("f1ae1dcd-76cd-4714-9df2-1ab0ebd2ca41")
}

fn yavimaya_kavu() -> CardIndex {
    card_index("285c9f49-4aeb-468c-bb41-f5ccb5257e2b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Crosis, the Purger (f1ae1dcd-76cd-4714-9df2-1ab0ebd2ca41) — `{3}{U}{B}{R}` 6/6
/// Legendary Creature — Dragon:
/// - Flying
/// - Whenever Crosis deals combat damage to a player, you may pay `{2}{B}`. If you do,
///   choose a color, then that player reveals their hand and discards all cards of that color.
#[test]
fn crosis_the_purger_pays_two_black_on_combat_damage_to_discard_all_cards_of_chosen_color() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[island(), swamp(), mountain(), swamp(), swamp(), swamp()],
        )
        .hand(0, &[crosis_the_purger()])
        .hand(
            1,
            &[grizzly_bears(), yavimaya_kavu(), lightning_bolt(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, crosis_the_purger());
    pass_until(&mut engine, stack_is_empty);
    let crosis = on_battlefield(&engine, P0, crosis_the_purger()).expect("Crosis entered");
    assert_eq!(pt(&engine, crosis), (6, 6));
    assert!(keywords(&engine, crosis).contains(KeywordSet::FLYING));

    // Combat 1: pay `{2}{B}`, choose Green -> discards mono-green Bears AND multicolor RG Kavu,
    // keeping red Lightning Bolt and colorless Forest.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let _ = attack_and_collect_blocks(&mut engine, crosis, P1);

    pass_until(&mut engine, |e| {
        !stack_is_empty(e) && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    tap_all_mana(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 14);
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
    assert!(
        in_graveyard(&engine, P1, yavimaya_kavu()).is_some(),
        "multicolored card of chosen color is also discarded"
    );
    assert!(in_hand(&engine, P1, lightning_bolt()).is_some());
    assert!(in_hand(&engine, P1, forest()).is_some());

    // Combat 2: declining `{2}{B}` leaves P1's remaining hand intact.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let _ = attack_and_collect_blocks(&mut engine, crosis, P1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(in_hand(&engine, P1, lightning_bolt()).is_some());
}
