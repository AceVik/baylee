//! Native graveyard alternatives coexist with the new permission stage.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch45_disturb_can_announce_its_back_face_before_making_the_mana() {
    let mut e = prepared(
        &[index::MALEVOLENT_HERMIT],
        &[index::ISLAND, index::ISLAND, index::ISLAND],
        &[],
    );
    let hermit = hand_to_graveyard(&mut e, P0, index::MALEVOLENT_HERMIT);
    pool(&mut e, ManaColor::Blue, 0);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority");
    };
    assert!(!legal.castable.contains(&hermit));
    assert!(
        legal.payable.contains(&hermit),
        "native disturb remains a graveyard route"
    );
    cast(&mut e, hermit);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected casting mana window");
    };
    assert!(legal.castable.is_empty());
    let islands = legal.mana_abilities.clone();
    assert_eq!(islands.len(), 3);
    for source in islands {
        e.apply(P0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    let spell = e.state.object(hermit).unwrap();
    assert_eq!(spell.zone, Zone::Stack);
    assert_eq!(spell.face_index, 1);
    assert_eq!(spell.characteristics().mana_value(), 2);
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        0,
        "disturb costs 2U, not the front's 1U"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(hermit).unwrap().zone, Zone::Battlefield);
    kill(&mut e, hermit);
    assert_eq!(e.state.object(hermit).unwrap().zone, Zone::Exile);
}
