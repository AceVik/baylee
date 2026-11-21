//! `cards/instants/mv_2/tangle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "f627e125-15af-4e53-b34e-82b60e4ec87b"
fn tangle() -> CardIndex {
    card_index("f627e125-15af-4e53-b34e-82b60e4ec87b")
}

/// Tangle ({1}{G}): prevents all combat damage this turn (not noncombat), keeps
/// every attacking creature tapped through its controller's next untap step
/// while non-attacking tapped creatures untap normally, and releases attackers
/// on the untap step after that.
#[test]
fn tangle_prevents_combat_damage_and_freezes_only_attackers_for_one_untap() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), mountain()])
        .hand(0, &[tangle(), shock()])
        .battlefield(
            1,
            &[
                index::HILL_GIANT,
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let elf = lk::obj(&engine, P1, index::LLANOWAR_ELVES);

    // Tap the non-attacking Elf for mana so we can observe it untapping next turn.
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: elf,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, elf));

    let _ = lk::attack(&mut engine, &[giant, bears]);
    lk::block(&mut engine, &[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    let forests = all_on_battlefield(&engine, P0, forest());
    tap_mana_where(&mut engine, P0, |id| forests.contains(&id));
    cast_with_floating(&mut engine, P0, tangle());
    pass_until(&mut engine, stack_is_empty);

    // Noncombat damage is still dealt on the Tangle turn.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    let mtn = lk::obj(&engine, P0, mountain());
    tap_mana_where(&mut engine, P0, |id| id == mtn);
    lk::cast_at(&mut engine, P0, shock(), &[], &[P1]);
    assert_eq!(
        lk::life(&engine, P1),
        18,
        "noncombat damage is not prevented"
    );

    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P0), 20, "all combat damage was prevented");

    // On P1's next turn: non-attacking Elf untaps; both attackers stay tapped.
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    assert!(!is_tapped(&engine, elf), "non-attacker untaps normally");
    assert!(
        is_tapped(&engine, giant) && is_tapped(&engine, bears),
        "attacking creatures do not untap during their controller's next untap step"
    );

    // On P1's following turn: the one-untap freeze has worn off.
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    assert!(
        !is_tapped(&engine, giant) && !is_tapped(&engine, bears),
        "attackers untap normally on the untap step after that"
    );
}
