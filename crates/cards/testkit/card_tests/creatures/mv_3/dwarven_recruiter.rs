//! `cards/creatures/mv_3/dwarven_recruiter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn dwarven_recruiter() -> CardIndex {
    card_index("d047a819-5196-4a16-8d67-a4edaf6fdef0")
}

/// Dwarven Recruiter's ETB finds any number of Dwarf cards from your library
/// (excluding non-Dwarves) and puts them on top in the chosen order.
#[test]
fn dwarven_recruiter_tutors_dwarves_to_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, index::DWARVEN_WARRIORS)
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[dwarven_recruiter(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let non_dwarf = hand_to_library_top(&mut engine, p0, grizzly_bears());

    cast_from_hand(&mut engine, p0, dwarven_recruiter());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected Dwarf search choice, got {:?}", engine.pending());
    };
    assert!(options.len() >= 2);
    assert!(
        !options.contains(&non_dwarf),
        "non-Dwarf card in library must be excluded from Dwarf search options"
    );
    assert!(options.iter().all(|&id| {
        engine
            .state()
            .object(id)
            .unwrap()
            .card
            .is_some_and(|c| c.index == index::DWARVEN_WARRIORS)
    }));
    let chosen = vec![options[0], options[1]];
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: chosen.clone(),
            },
        )
        .unwrap();
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(&mut engine, |e| at_rest(e, p0));

    let lib = engine.state().zones.list(ZoneLocation::Library(p0));
    let top_two: Vec<_> = lib.iter().rev().take(2).copied().collect();
    assert!(top_two.contains(&chosen[0]));
    assert!(top_two.contains(&chosen[1]));
}
