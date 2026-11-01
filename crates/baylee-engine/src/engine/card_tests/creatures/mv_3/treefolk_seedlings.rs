#[allow(clippy::wildcard_imports)]
use super::*;

fn treefolk_seedlings() -> CardIndex {
    card_index("87285539-29c8-4cc7-a22c-4a4b96a61e82")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn treefolk_seedlings_tracks_only_your_forests_as_lands_are_played() {
    let p0 = PlayerId::new(0);
    let mut e = lk::game(
        SEED,
        &[forest(), island()],
        &[forest(), forest()],
        &[treefolk_seedlings(), forest()],
        &[],
    );
    lk::float(&mut e, p0, &[(ManaColor::Green, 3)]);
    cast_with_floating(&mut e, p0, treefolk_seedlings());
    lk::settle(&mut e);
    let seedlings = lk::obj(&e, p0, treefolk_seedlings());
    assert_eq!(pt(&e, seedlings), (2, 1));
    let land = in_hand(&e, p0, forest()).unwrap();
    e.apply(p0, PlayerAction::PlayLand { card: land }).unwrap();
    assert_eq!(pt(&e, seedlings), (2, 2));
}
