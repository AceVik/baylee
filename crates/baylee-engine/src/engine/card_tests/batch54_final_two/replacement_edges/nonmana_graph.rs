//! New child of `batch54_final_two::replacement_edges` (inherits fixture helpers).
#[allow(clippy::wildcard_imports)]
use super::*;
static TARGETED_MANA: baylee_cards_dsl::AbilityDef = baylee_cards_dsl::activated!(
    Cost::TAP,
    &[Effect::mana(ManaColor::White, 2)],
    target = Some(baylee_cards_dsl::TargetSpec::Object(&Filter::CREATURE))
);
#[test]
fn gnomes_does_not_rewrite_targeted_nonmana_ability_or_its_reflected_type() {
    let mut e = prepared(
        &[],
        &[
            index::QUARUM_TRENCH_GNOMES,
            index::PLAINS,
            index::REFLECTING_POOL,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let land = on_battlefield(&e, P0, index::PLAINS).unwrap();
    curse(&mut e, land);
    // Transparent definition fixture: a target makes this activated ability
    // nonmana despite its output; its tap symbol alone is not tapped-for-mana.
    affect(
        &mut e,
        land,
        Modifier::GrantActivatedDefinition(&TARGETED_MANA),
    );
    let grant = crate::effects::granted_activated(e.state(), land)
        .next()
        .unwrap()
        .id;
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: grant,
        },
    )
    .unwrap();
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    target(&mut e, bear);
    assert!(!stack_is_empty(&e));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 2);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        0
    );
    reset(&mut e);
    let pool = on_battlefield(&e, P0, index::REFLECTING_POOL).unwrap();
    tap_source(&mut e, P0, pool);
    let Pending::ChooseColor { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert_eq!(options.len(), 2);
    assert!(options.contains(&ManaColor::White) && options.contains(&ManaColor::Colorless));
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 1);
}
