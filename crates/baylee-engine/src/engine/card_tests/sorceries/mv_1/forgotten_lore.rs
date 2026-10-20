//! `cards/sorceries/mv_1/forgotten_lore.rs`, played.
//!
//! Oracle:
//! - "Target opponent chooses a card in your graveyard. You may pay {G}. If you do, repeat this process except that opponent can't choose a card already chosen for Forgotten Lore. Then put the last chosen card into your hand."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn forgotten_lore() -> CardIndex {
    card_index("ca9f538d-4191-448a-870d-f645b7c37c1b")
}

#[test]
fn forgotten_lore_returns_card_from_graveyard_to_hand() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[index::LIGHTNING_BOLT, forgotten_lore()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[bears], &[]);
    let dead_bears = in_graveyard(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Green, 1)]);
    lk::cast_at(&mut engine, P0, forgotten_lore(), &[dead_bears], &[]);

    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
