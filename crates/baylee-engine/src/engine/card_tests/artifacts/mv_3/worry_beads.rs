//! Oracle: at the beginning of each player's upkeep, that player mills a card.
#[allow(clippy::wildcard_imports)]
use super::*;
fn worry_beads() -> CardIndex {
    card_index("ab050d55-a47d-4880-b65a-29f562773d7a")
}
/// Checklist: upkeep trigger fires for each active player; each mills own library's top card.
#[test]
fn mills_active_player_at_both_upkeeps() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    let mut g = Duel::new(SEED, forest())
        .battlefield(0, &[worry_beads()])
        .start();
    keep_mulligans(&mut g);
    let l0 = library_size(&g, p);
    let l1 = library_size(&g, q);
    reach_main_phase(&mut g, p);
    assert_eq!(
        library_size(&g, p),
        l0 - 1,
        "the starting player mills at upkeep and has no first-turn draw"
    );
    assert_eq!(library_size(&g, q), l1);
    assert_eq!(g.state().zones.list(ZoneLocation::Graveyard(p)).len(), 1);
    reach_their_main_phase(&mut g, q);
    assert_eq!(library_size(&g, p), l0 - 1);
    assert_eq!(g.state().zones.list(ZoneLocation::Graveyard(q)).len(), 1);
    assert_eq!(
        library_size(&g, q),
        l1 - 2,
        "p1 mills at upkeep and draws for turn"
    );
}
