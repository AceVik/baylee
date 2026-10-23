#[allow(clippy::wildcard_imports)]
use super::*;

fn rain_of_filth() -> CardIndex {
    card_index("aa77980d-0434-40f0-990f-1c8c50610174")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn rain_of_filth_grants_each_existing_owned_land_a_temporary_sacrifice_mana_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[swamp(), forest()],
        &[island()],
        &[rain_of_filth(), forest()],
        &[],
    );
    let mine = lk::obj(&e, p0, swamp());
    let other = lk::obj(&e, p0, forest());
    let theirs = lk::obj(&e, p1, island());
    lk::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut e, p0, rain_of_filth());
    lk::settle(&mut e);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    let grants: Vec<_> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(id, _)| *id == mine || *id == other)
        .collect();
    assert_eq!(grants.len(), 2);
    assert!(!legal.abilities.iter().any(|(id, _)| *id == theirs));
    let later = in_hand(&e, p0, forest()).unwrap();
    e.apply(p0, PlayerAction::PlayLand { card: later }).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.abilities.iter().any(|(id, _)| *id == later));
    let (source, ability_index) = grants[0];
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .unwrap();
    assert_eq!(lk::pool_of(&e, p0, ManaColor::Black), 1);
    assert!(stack_is_empty(&e), "a mana ability resolves immediately");
    assert_eq!(
        e.state().object(source).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, _)| *id == mine || *id == other)
    );
}
