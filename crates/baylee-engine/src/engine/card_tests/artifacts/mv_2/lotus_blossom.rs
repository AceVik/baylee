#[allow(clippy::wildcard_imports)]
use super::*;

fn lotus_blossom() -> CardIndex {
    card_index("32e66ea0-5f35-4df8-8d00-580403c51bb5")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn lotus_blossom_counts_only_accepted_own_upkeeps_and_makes_exact_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game_with(SEED, &[lotus_blossom()], &[], &[], &[], |e| {
        lk::yes_no(e, false);
    });
    let blossom = lk::obj(&e, p0, lotus_blossom());
    assert_eq!(
        counters_on(&e, blossom, baylee_cards_dsl::counters::PETAL),
        0
    );
    reach_their_main_phase(&mut e, p1);
    assert_eq!(
        counters_on(&e, blossom, baylee_cards_dsl::counters::PETAL),
        0
    );
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    reach_main_phase(&mut e, p0);
    assert_eq!(
        counters_on(&e, blossom, baylee_cards_dsl::counters::PETAL),
        1
    );
    activate(&mut e, p0, lotus_blossom(), 1);
    let Pending::ChooseColor { options, .. } = e.pending() else {
        panic!("choose one color")
    };
    assert_eq!(options.len(), 5);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(lk::pool_of(&e, p0, ManaColor::Green), 1);
    assert_eq!(lk::pool_total(&e, p0), 1);
    assert!(in_graveyard(&e, p0, lotus_blossom()).is_some());
}

#[test]
fn lotus_blossom_without_petals_makes_no_mana() {
    let p0 = PlayerId::new(0);
    let mut e = lk::game_with(SEED, &[lotus_blossom()], &[], &[], &[], |e| {
        lk::yes_no(e, false);
    });
    activate(&mut e, p0, lotus_blossom(), 1);
    if matches!(e.pending(), Pending::ChooseColor { .. }) {
        e.apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
    }
    assert_eq!(lk::pool_total(&e, p0), 0);
    assert!(in_graveyard(&e, p0, lotus_blossom()).is_some());
}
