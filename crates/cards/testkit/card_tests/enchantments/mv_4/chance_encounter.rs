//! `cards/enchantments/mv_4/chance_encounter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn chance_encounter() -> CardIndex {
    card_index("8bd33b5c-0777-41ef-9e17-fb30ce69ec25")
}

fn frenetic_efreet() -> CardIndex {
    card_index("c21d3360-8be0-477e-a592-7e64a76570d3")
}

/// Winning coin flips adds luck counters to Chance Encounter; <10 counters
/// does not win on your upkeep, while 10+ counters wins at your upkeep.
#[test]
fn chance_encounter_counts_won_flips_and_wins_at_upkeep_with_ten_luck_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[chance_encounter(), frenetic_efreet()],
        &[],
        &[frenetic_efreet(), frenetic_efreet()],
        &[],
    );
    let enc = lk::obj(&e, p0, chance_encounter());

    // Stack 8 coin-flip activations on the first Efreet -> between 1 and 8 luck counters (< 10).
    for _ in 0..8 {
        activate(&mut e, p0, frenetic_efreet(), 0);
    }
    lk::settle(&mut e);
    let initial_luck = e.state().object(enc).unwrap().counters.get(counters::LUCK);
    assert!((1..10).contains(&initial_luck));

    // Reaching P0's next upkeep with < 10 luck counters does NOT win.
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert!(e.state().game_end.is_none());

    // Cast fresh Frenetic Efrets in batches (<100 stack passes per settle) -> >= 10 luck counters.
    for _ in 0..2 {
        if e.state().object(enc).unwrap().counters.get(counters::LUCK) >= 10 {
            break;
        }
        lk::float(
            &mut e,
            p0,
            &[
                (ManaColor::Colorless, 1),
                (ManaColor::Blue, 1),
                (ManaColor::Red, 1),
            ],
        );
        lk::cast_at(&mut e, p0, frenetic_efreet(), &[], &[]);
        for _ in 0..20 {
            activate(&mut e, p0, frenetic_efreet(), 0);
        }
        lk::settle(&mut e);
    }
    assert!(e.state().object(enc).unwrap().counters.get(counters::LUCK) >= 10);

    // Next P0 upkeep wins the game via EffectWin.
    reach_their_main_phase(&mut e, p1);
    pass_until(&mut e, |e| e.state().game_end.is_some());
    assert_eq!(
        e.state().game_end,
        Some(baylee_engine::win::GameResult {
            winner: Some(baylee_engine::win::Victor::Player(p0)),
            reason: baylee_engine::win::EndReason::EffectWin,
        })
    );
}
