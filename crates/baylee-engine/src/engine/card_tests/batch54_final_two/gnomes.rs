#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch54_gnomes_replaces_white_indefinitely_but_not_new_incarnation() {
    let mut e = prepared(
        &[index::UNSUMMON],
        &[index::QUARUM_TRENCH_GNOMES, index::PLAINS],
        &[],
    );
    let plains = on_battlefield(&e, P0, index::PLAINS).unwrap();
    activate(&mut e, P0, index::QUARUM_TRENCH_GNOMES, 0);
    target(&mut e, plains);
    pass_until(&mut e, stack_is_empty);
    let gnomes = on_battlefield(&e, P0, index::QUARUM_TRENCH_GNOMES).unwrap();
    let bounce = in_hand(&e, P0, index::UNSUMMON).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: bounce })
        .unwrap();
    target(&mut e, gnomes);
    pass_until(&mut e, stack_is_empty);
    reset(&mut e);
    tap_source(&mut e, P0, plains);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        1
    );
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 0);
    next_main(&mut e, P0);
    reset(&mut e);
    tap_source(&mut e, P0, plains);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        1
    );
    // Transparent blink setup tests exact incarnation without an unrelated helper.
    e.state
        .move_object(
            plains,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            plains,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    reset(&mut e);
    tap_source(&mut e, P0, plains);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 1);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        0
    );
}
#[test]
fn batch54_gnomes_preserves_nonwhite_mana_from_granted_ability() {
    let mut e = prepared(
        &[],
        &[
            index::QUARUM_TRENCH_GNOMES,
            index::PLAINS,
            index::CHROMATIC_LANTERN,
        ],
        &[],
    );
    let plains = on_battlefield(&e, P0, index::PLAINS).unwrap();
    activate(&mut e, P0, index::QUARUM_TRENCH_GNOMES, 0);
    target(&mut e, plains);
    pass_until(&mut e, stack_is_empty);
    reset(&mut e);
    let grant = crate::effects::granted_activated(e.state(), plains)
        .next()
        .unwrap()
        .id;
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: plains,
            ability_index: grant,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Blue), 1);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        0
    );
}
