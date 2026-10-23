//! `cards/creatures/mv_4/mistmoon_griffin.rs`, played.
//!
//! The unsupported top-card return is omitted; the mandatory self-exile
//! on death remains so the body does not lose its printed drawback.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);

fn mistmoon_griffin() -> CardIndex {
    card_index("7fe9d30f-c778-4769-a3ae-3cb48cd23e07")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Mistmoon Griffin (7fe9d30f-c778-4769-a3ae-3cb48cd23e07) — "Flying", a 2/2.
/// Bolted, it dies and its trigger exiles it; the unsupported return does
/// not bring the Grizzly Bears in its graveyard back.
#[test]
fn mistmoon_griffin_exiles_itself_when_it_dies_without_returning_an_arbitrary_creature() {
    let mut e = lk::game(
        SEED,
        &[mistmoon_griffin()],
        &[],
        &[lightning_bolt(), grizzly_bears()],
        &[],
    );
    let griffin = lk::obj(&e, P0, mistmoon_griffin());
    hand_to_graveyard(&mut e, P0, grizzly_bears());
    assert_eq!(pt(&e, griffin), (2, 2));
    assert!(keywords(&e, griffin).contains(KeywordSet::FLYING));

    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, lightning_bolt());
    aim_at(&mut e, P0, griffin);
    lk::settle(&mut e);

    assert!(
        in_graveyard(&e, P0, mistmoon_griffin()).is_none(),
        "the dies trigger exiles it"
    );
    assert_eq!(lk::zone_of(&e, griffin), Zone::Exile);
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "nothing came back"
    );
    assert!(on_battlefield(&e, P0, grizzly_bears()).is_none());
}
