//! `cards/sorceries/mv_2/mana_severance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn mana_severance() -> CardIndex {
    card_index("34c66f52-dcbf-4d0a-bf7f-cb294bd93e9c")
}

/// Mana Severance (`34c66f52-dcbf-4d0a-bf7f-cb294bd93e9c`) — {1}{U} Sorcery (Partial):
/// "Search your library for any number of land cards, exile them, then shuffle."
#[test]
fn mana_severance_searches_library_for_land_cards() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[mana_severance()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mana_severance());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards");
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, forest()).is_some());
}
