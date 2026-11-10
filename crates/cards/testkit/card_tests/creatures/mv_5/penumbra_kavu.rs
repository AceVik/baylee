//! Oracle scenarios for Penumbra Kavu.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};

fn penumbra_kavu() -> CardIndex {
    card_index("0abcf934-2552-4885-bbe7-38d93ce0e642")
}

#[test]
fn penumbra_kavu_creates_3_3_black_kavu_token_when_it_dies() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::FOREST,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
            ],
        )
        .hand(0, &[penumbra_kavu(), index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, penumbra_kavu());
    pass_until(&mut e, stack_is_empty);
    let kavu = on_battlefield(&e, p0, penumbra_kavu()).unwrap();
    assert_eq!(pt(&e, kavu), (3, 3));

    cast_from_hand(&mut e, p0, index::LIGHTNING_BOLT);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![kavu],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, penumbra_kavu()).is_some());
    let token = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            let o = e.state().object(id).unwrap();
            o.controller == p0 && o.card.is_none()
        })
        .expect("3/3 black Kavu token created");
    assert_eq!(pt(&e, token), (3, 3));
    let chars = e.state().object(token).unwrap().characteristics();
    assert!(chars.colors.contains(baylee_core::color::Color::Black));
    assert!(chars.subtypes.contains(subtypes::creature::KAVU));
}
