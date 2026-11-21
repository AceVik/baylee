//! `cards/sorceries/mv_6/rain_of_salt.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn rain_of_salt() -> CardIndex {
    card_index("1219e330-01ac-405a-b75a-dd4298598167")
}

/// Rain of Salt (1219e330-01ac-405a-b75a-dd4298598167) — {4}{R}{R} Sorcery:
/// "Destroy two target lands."
#[test]
fn rain_of_salt_destroys_two_target_lands_and_excludes_nonlands() {
    let mut engine = lk::game(
        SEED,
        &[
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            sol_ring(),
        ],
        &[island(), forest(), plains(), aurochs()],
        &[rain_of_salt()],
        &[],
    );
    let ring = lk::obj(&engine, P0, sol_ring());
    let their_island = lk::obj(&engine, P1, island());
    let their_forest = lk::obj(&engine, P1, forest());
    let their_plains = lk::obj(&engine, P1, plains());
    let their_beast = lk::obj(&engine, P1, aurochs());

    tap_mana_except(&mut engine, P0, ring);
    cast_with_floating(&mut engine, P0, rain_of_salt());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2));
    assert!(
        options.contains(&their_island)
            && options.contains(&their_forest)
            && options.contains(&their_plains)
    );
    assert!(!options.contains(&ring) && !options.contains(&their_beast));

    lk::aim(&mut engine, &[their_island, their_forest], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, their_island), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_forest), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_plains), Zone::Battlefield);
}

/// Rain of Salt — requires at least two lands on the battlefield to be cast.
#[test]
fn rain_of_salt_cannot_be_cast_with_fewer_than_two_lands_in_play() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), sol_ring()],
        &[aurochs()],
        &[rain_of_salt()],
        &[],
    );
    lk::float(&mut engine, P0, &[(ManaColor::Red, 6)]);

    assert!(!lk::castable(&engine, P0, rain_of_salt()));
}
