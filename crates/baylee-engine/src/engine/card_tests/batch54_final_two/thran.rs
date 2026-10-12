#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch54_thran_all_five_choices_preserve_gate_and_tax_intrinsic_mana() {
    for (subtype, color) in [
        (subtypes::land::PLAINS, ManaColor::White),
        (subtypes::land::ISLAND, ManaColor::Blue),
        (subtypes::land::SWAMP, ManaColor::Black),
        (subtypes::land::MOUNTAIN, ManaColor::Red),
        (subtypes::land::FOREST, ManaColor::Green),
    ] {
        let mut e = prepared(&[index::THRAN_PORTAL], &[], &[]);
        reset(&mut e);
        let portal = play_land(&mut e, P0, index::THRAN_PORTAL);
        let Pending::ChooseSubtype { options, .. } = e.pending() else {
            panic!("{:?}", e.pending())
        };
        assert_eq!(options.len(), 5);
        assert!(options.contains(&subtype));
        e.apply(P0, PlayerAction::ChooseSubtype(subtype)).unwrap();
        let chars = e.state.object(portal).unwrap().characteristics();
        assert!(chars.subtypes.contains(subtype));
        assert!(chars.subtypes.contains(subtypes::land::GATE));
        assert!(
            !chars
                .supertypes
                .contains(baylee_core::types::SupertypeSet::BASIC)
        );
        let life = e.state.players[0].life;
        tap_source(&mut e, P0, portal);
        assert_eq!(e.state.players[0].life, life - 1);
        assert_eq!(e.state.players[0].mana_pool.available(color), 1);
        assert!(is_tapped(&e, portal));
    }
}
#[test]
fn batch54_thran_lantern_grant_is_also_taxed() {
    let mut e = prepared(&[index::THRAN_PORTAL], &[index::CHROMATIC_LANTERN], &[]);
    reset(&mut e);
    let portal = play_land(&mut e, P0, index::THRAN_PORTAL);
    e.apply(P0, PlayerAction::ChooseSubtype(subtypes::land::FOREST))
        .unwrap();
    let grant = crate::effects::granted_activated(e.state(), portal)
        .next()
        .unwrap()
        .id;
    let life = e.state.players[0].life;
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: portal,
            ability_index: grant,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(e.state.players[0].life, life - 1);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Blue), 1);
}
