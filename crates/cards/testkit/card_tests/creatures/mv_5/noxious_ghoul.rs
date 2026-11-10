//! `cards/creatures/mv_5/noxious_ghoul.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Noxious Ghoul (`602e421e-afab-4488-8a52-af66b4fdce4b`) — {3}{B}{B} Creature — Zombie 3/3:
/// "Whenever this creature or another Zombie enters, all non-Zombie creatures get -1/-1 until end of turn."
#[test]
fn noxious_ghoul_shrinks_non_zombies_when_it_or_another_zombie_enters() {
    let ghoul_card = card_index("602e421e-afab-4488-8a52-af66b4fdce4b");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                index::SCATHE_ZOMBIES,
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[ghoul_card, index::SCATHE_ZOMBIES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let zombie = on_battlefield(&engine, p0, index::SCATHE_ZOMBIES).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, ghoul_card);
    pass_until(&mut engine, stack_is_empty);

    let ghoul = on_battlefield(&engine, p0, ghoul_card).unwrap();
    assert_eq!(pt(&engine, ghoul), (3, 3));
    assert_eq!(pt(&engine, zombie), (2, 2));
    assert_eq!(pt(&engine, bear), (1, 1));

    // Cast another Zombie; a second -1/-1 kills the 1/1 Grizzly Bears.
    cast_with_floating(&mut engine, p0, index::SCATHE_ZOMBIES);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(pt(&engine, ghoul), (3, 3));
}
