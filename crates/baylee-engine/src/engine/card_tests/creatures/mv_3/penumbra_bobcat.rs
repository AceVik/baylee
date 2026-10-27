//! Oracle scenarios for Penumbra Bobcat.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};

fn penumbra_bobcat() -> CardIndex {
    card_index("f0787397-8edb-4794-8ee1-22f97a057be8")
}

#[test]
fn penumbra_bobcat_creates_2_1_black_cat_token_when_it_dies() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::FOREST,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
            ],
        )
        .hand(0, &[penumbra_bobcat(), index::SHOCK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, penumbra_bobcat());
    pass_until(&mut e, stack_is_empty);
    let bobcat = on_battlefield(&e, p0, penumbra_bobcat()).unwrap();
    assert_eq!(pt(&e, bobcat), (2, 1));

    cast_from_hand(&mut e, p0, index::SHOCK);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bobcat],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, penumbra_bobcat()).is_some());
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
        .expect("2/1 black Cat token created");
    assert_eq!(pt(&e, token), (2, 1));
    let chars = e.state().object(token).unwrap().characteristics();
    assert!(chars.colors.contains(baylee_core::color::Color::Black));
    assert!(chars.subtypes.contains(subtypes::creature::CAT));
}
