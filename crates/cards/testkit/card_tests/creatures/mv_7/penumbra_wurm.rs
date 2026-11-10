//! Oracle scenarios for Penumbra Wurm.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};

fn penumbra_wurm() -> CardIndex {
    card_index("abccb19f-098b-4453-b7bc-838ad5914ebe")
}

#[test]
fn penumbra_wurm_has_trample_and_creates_6_6_black_trample_wurm_token_when_it_dies() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::FOREST,
                index::FOREST,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
            ],
        )
        .hand(0, &[penumbra_wurm(), index::TERROR])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, penumbra_wurm());
    pass_until(&mut e, stack_is_empty);
    let wurm = on_battlefield(&e, p0, penumbra_wurm()).unwrap();
    assert_eq!(pt(&e, wurm), (6, 6));
    assert!(keywords(&e, wurm).contains(KeywordSet::TRAMPLE));

    cast_from_hand(&mut e, p0, index::TERROR);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![wurm],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, penumbra_wurm()).is_some());
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
        .expect("6/6 black Wurm token created");
    assert_eq!(pt(&e, token), (6, 6));
    assert!(keywords(&e, token).contains(KeywordSet::TRAMPLE));
    let chars = e.state().object(token).unwrap().characteristics();
    assert!(chars.colors.contains(baylee_core::color::Color::Black));
    assert!(chars.subtypes.contains(subtypes::creature::WURM));
}
