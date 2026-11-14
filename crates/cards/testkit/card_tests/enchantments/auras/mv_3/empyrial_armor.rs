//! `cards/1997/src/wth/empyrial_armor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn empyrial_armor() -> CardIndex {
    card_index("ec9e8611-e785-4d5d-b6e1-bf63daf408df")
}

/// Only creatures can be enchanted, and Empyrial Armor's +1/+1 tracks its
/// controller's hand size dynamically as cards leave and enter their hand.
#[test]
fn empyrial_armor_targets_creatures_and_scales_with_your_hand_size() {
    an_aura_enchants_only_creatures(empyrial_armor(), plains());

    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), grizzly_bears()],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[empyrial_armor(), sol_ring(), forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    let plains_ids = all_on_battlefield(&engine, P0, plains());
    tap_mana_where(&mut engine, P0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, P0, empyrial_armor());
    aim_at(&mut engine, P0, my_bears);
    pass_until(&mut engine, stack_is_empty);

    // 3 cards left in P0's hand -> +3/+3 on 2/2 Bears = 5/5.
    let hand_after_armor = engine.state().zones.list(ZoneLocation::Hand(P0)).len() as i16;
    assert_eq!(
        pt(&engine, my_bears),
        (2 + hand_after_armor, 2 + hand_after_armor)
    );
    assert_eq!(pt(&engine, their_bears), (2, 2));

    // Cast Sol Ring ({1} from floating Plains): hand shrinks by 1 -> P/T drops by 1/1.
    cast_with_floating(&mut engine, P0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, my_bears),
        (1 + hand_after_armor, 1 + hand_after_armor)
    );

    // Advance to P0's next main phase (draws 1 card): hand grows by 1 -> P/T rises by 1/1.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(
        pt(&engine, my_bears),
        (2 + hand_after_armor, 2 + hand_after_armor)
    );
}

/// When attached to an opponent's creature, "your hand" still counts the Aura
/// controller's hand (not the enchanted creature's controller).
#[test]
fn empyrial_armor_on_opponents_creature_counts_aura_controllers_hand() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 3])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[empyrial_armor(), forest()])
        .hand(1, &[island(), mountain(), swamp(), plains(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, empyrial_armor());
    aim_at(&mut engine, P0, their_bears);
    pass_until(&mut engine, stack_is_empty);

    // P0 has 1 card left in hand; P1 has 5 cards. Bonus must be +1/+1, not +5/+5.
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P0)).len(), 1);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P1)).len(), 5);
    assert_eq!(pt(&engine, their_bears), (3, 3));

    // Play P0's last card (Forest) -> P0 hand = 0 -> their Bears drop to 2/2.
    play_land(&mut engine, P0, forest());
    assert_eq!(pt(&engine, their_bears), (2, 2));

    // P1 draws on P1's turn -> P1 has 6 cards, P0 still has 0 -> Bears remain 2/2.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, their_bears), (2, 2));
}
