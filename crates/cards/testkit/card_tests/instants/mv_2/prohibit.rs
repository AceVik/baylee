//! `cards/instants/mv_2/prohibit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn prohibit() -> CardIndex {
    card_index("d95f1797-56f9-41be-a5fe-8398961f4b8b")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Prohibit ({1}{U} Instant, Kicker {2}):
/// - Unkicked counters MV 2 spell, does not counter MV 4 spell even though it targets it.
/// - Kicked counters MV 4 spell.
#[test]
fn prohibit_counters_mv_two_unkicked_and_mv_four_kicked() {
    // Unkicked vs MV 2 Grizzly Bears -> countered.
    let mut e1 = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[prohibit()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut e1);
    reach_their_main_phase(&mut e1, P1);
    cast_from_hand(&mut e1, P1, grizzly_bears());
    let bears_spell = e1.state().zones.list(ZoneLocation::Stack)[0];
    e1.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut e1, P0);
    cast_with_floating(&mut e1, P0, prohibit());
    lk::aim(&mut e1, &[bears_spell], &[]);
    pass_until(&mut e1, stack_is_empty);
    assert!(in_graveyard(&e1, P1, grizzly_bears()).is_some());
    assert!(on_battlefield(&e1, P1, grizzly_bears()).is_none());

    // Unkicked vs MV 4 Hill Giant -> resolves anyway.
    let mut e2 = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[forest(), forest(), forest(), forest()])
        .hand(0, &[prohibit()])
        .hand(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut e2);
    reach_their_main_phase(&mut e2, P1);
    // Float 4 red/generic via sol rings? Use mountain/island + floating:
    lk::float(&mut e2, P1, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e2, P1, hill_giant());
    let giant_spell = e2.state().zones.list(ZoneLocation::Stack)[0];
    e2.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut e2, P0);
    cast_with_floating(&mut e2, P0, prohibit());
    e2.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::aim(&mut e2, &[giant_spell], &[]);
    pass_until(&mut e2, stack_is_empty);
    assert!(on_battlefield(&e2, P1, hill_giant()).is_some());

    // Kicked vs MV 4 Hill Giant -> countered.
    let mut e3 = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[prohibit()])
        .hand(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut e3);
    reach_their_main_phase(&mut e3, P1);
    lk::float(&mut e3, P1, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e3, P1, hill_giant());
    let giant_spell3 = e3.state().zones.list(ZoneLocation::Stack)[0];
    e3.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut e3, P0);
    cast_with_floating(&mut e3, P0, prohibit());
    e3.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::aim(&mut e3, &[giant_spell3], &[]);
    pass_until(&mut e3, stack_is_empty);
    assert!(in_graveyard(&e3, P1, hill_giant()).is_some());
    assert!(on_battlefield(&e3, P1, hill_giant()).is_none());
}
