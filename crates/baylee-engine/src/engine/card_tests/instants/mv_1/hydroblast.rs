//! `cards/instants/mv_1/hydroblast.rs`, played.
//!
//! Oracle:
//! - "Choose one —"
//! - "• Counter target spell if it's red."
//! - "• Destroy target permanent if it's red."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hydroblast() -> CardIndex {
    card_index("b94c0336-2ca8-4bb1-bb1f-e9babd1826ad")
}

#[test]
fn hydroblast_destroys_red_permanent_and_leaves_nonred_permanent_alone() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[hydroblast(), hydroblast()],
        &[],
    );
    let red_giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let green_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    lk::cast_modal(&mut engine, P0, hydroblast(), 1, 0, &[red_giant], &[]);
    assert_eq!(lk::zone_of(&engine, red_giant), Zone::Graveyard);

    lk::cast_modal(&mut engine, P0, hydroblast(), 1, 0, &[green_bears], &[]);
    assert_eq!(
        lk::zone_of(&engine, green_bears),
        Zone::Battlefield,
        "Hydroblast does not destroy a non-red permanent"
    );
}
