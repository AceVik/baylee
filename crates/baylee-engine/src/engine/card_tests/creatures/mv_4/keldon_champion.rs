//! `cards/creatures/mv_4/keldon_champion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn keldon_champion() -> CardIndex {
    card_index("1600b361-9298-49f0-8925-4c1aad363e1b")
}

/// Keldon Champion (1600b361-9298-49f0-8925-4c1aad363e1b) — "Haste / Echo
/// {2}{R}{R} / When this creature enters, it deals 3 damage to target player
/// or planeswalker."
#[test]
fn keldon_champion_deals_three_to_player_on_etb_attacks_with_haste_and_sacrifices_if_echo_unpaid() {
    let mut e = lk::game(SEED, &[], &[llanowar_elves()], &[keldon_champion()], &[]);
    let their_elf = lk::obj(&e, lk::P1, llanowar_elves());

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e, lk::P0, keldon_champion());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!("expected target choice");
    };
    assert!(
        !options.contains(&their_elf),
        "creatures are not legal targets"
    );
    assert!(player_options.contains(&lk::P1));
    lk::aim(&mut e, &[], &[lk::P1]);
    lk::settle(&mut e);

    assert_eq!(lk::life(&e, lk::P1), 17, "3 ETB damage to target player");
    let champ = lk::obj(&e, lk::P0, keldon_champion());
    assert!(keywords(&e, champ).contains(KeywordSet::HASTE));

    // Attacks the turn it enters thanks to haste.
    let _ = lk::attack(&mut e, &[champ]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 14, "3 more combat damage");

    // Next upkeep: decline Echo -> sacrificed.
    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 4)]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, lk::P0, keldon_champion()).is_some());
}

/// Keldon Champion — paying Echo {2}{R}{R} in the next upkeep keeps it on the
/// battlefield.
#[test]
fn keldon_champion_survives_when_echo_is_paid() {
    let mut e = lk::game(SEED, &[], &[], &[keldon_champion()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e, lk::P0, keldon_champion());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[], &[lk::P1]);
    lk::settle(&mut e);

    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 4)]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    reach_main_phase(&mut e, lk::P0);

    assert!(on_battlefield(&e, lk::P0, keldon_champion()).is_some());
}
