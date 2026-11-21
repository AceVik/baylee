//! `cards/instants/mv_5/scatter_the_seeds.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::{index, subtypes};

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "c17d5153-cc74-4fbb-9d75-f7ee7b557b0a"
fn scatter_the_seeds() -> CardIndex {
    card_index("c17d5153-cc74-4fbb-9d75-f7ee7b557b0a")
}

/// Scatter the Seeds ({3}{G}{G}, Convoke): convokes both colored ({G}{G}) and
/// generic ({1}) costs using untapped creatures, and creates three 1/1 green
/// Saproling creature tokens.
#[test]
fn scatter_the_seeds_convokes_and_creates_three_saproling_tokens() {
    let mut engine = lk::game(
        SEED,
        &[
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::HILL_GIANT,
        ],
        &[],
        &[scatter_the_seeds()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let elf = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);

    // Float only {2} colorless; convoke Bears ({G}) + Elf ({G}) + Giant ({1}).
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    cast_with_floating(&mut engine, P0, scatter_the_seeds());
    lk::aim(&mut engine, &[bears, elf, giant], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 0, "remaining {{2}} spent");
    assert!(is_tapped(&engine, bears) && is_tapped(&engine, elf) && is_tapped(&engine, giant));

    let tokens: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            let obj = engine.state().object(*id).unwrap();
            obj.controller == P0
                && obj.token.is_some()
                && obj
                    .characteristics()
                    .subtypes
                    .contains(subtypes::creature::SAPROLING)
        })
        .collect();
    assert_eq!(tokens.len(), 3, "created three Saproling tokens");
    for token in tokens {
        let ch = engine.state().object(token).unwrap().characteristics();
        assert_eq!(pt(&engine, token), (1, 1));
        assert_eq!(ch.colors, ColorSet::from_slice(&[Color::Green]));
    }
}
