#[allow(clippy::wildcard_imports)]
use super::*;

fn enchant(e: &mut Engine<RegistryLookup>, artifact: ObjectId) {
    cast_at(e, index::POWER_ARTIFACT, vec![artifact]);
}

#[test]
fn batch47_power_artifact_reduces_only_its_attachment_and_leaves_free_abilities_free() {
    for (card, price) in [(index::BASALT_MONOLITH, 1), (index::GRIM_MONOLITH, 2)] {
        let mut e = prepared(&[index::POWER_ARTIFACT], &[card, index::JAYEMDAE_TOME], &[]);
        let artifact = on_battlefield(&e, P0, card).unwrap();
        let tome = on_battlefield(&e, P0, index::JAYEMDAE_TOME).unwrap();
        enchant(&mut e, artifact);
        set_pool(&mut e, P0, &[]);
        activate_as(&mut e, P0, artifact, 1);
        assert_eq!(
            e.state.players[0].mana_pool.total(),
            3,
            "no one-mana floor is added to a free ability"
        );
        set_pool(&mut e, P0, &[(ManaColor::Colorless, price)]);
        assert!(
            e.apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: tome,
                    ability_index: 0
                }
            )
            .is_err()
        );
        activate_as(&mut e, P0, artifact, 2);
        assert_eq!(e.state.players[0].mana_pool.total(), 0);
        pass_until(&mut e, stack_is_empty);
        assert!(!is_tapped(&e, artifact));
    }
}

#[test]
fn batch47_power_artifact_applies_to_mana_abilities_and_two_reductions_keep_one_mana_floor() {
    let mut e = prepared(
        &[index::POWER_ARTIFACT, index::POWER_ARTIFACT],
        &[index::CELESTIAL_PRISM],
        &[],
    );
    let prism = on_battlefield(&e, P0, index::CELESTIAL_PRISM).unwrap();
    enchant(&mut e, prism);
    enchant(&mut e, prism);
    set_pool(&mut e, P0, &[]);
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: prism,
                ability_index: 0
            }
        )
        .is_err()
    );
    set_pool(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    activate_as(&mut e, P0, prism, 0);
    assert!(matches!(e.pending(), Pending::ChooseColor { .. }));
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert!(stack_is_empty(&e));
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Blue), 1);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        0
    );
}

#[test]
fn batch47_power_artifact_preserves_colored_requirements() {
    let mut e = prepared(
        &[index::POWER_ARTIFACT],
        &[index::PYRITE_SPELLBOMB],
        &[index::GRIZZLY_BEARS],
    );
    let bomb = on_battlefield(&e, P0, index::PYRITE_SPELLBOMB).unwrap();
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    enchant(&mut e, bomb);
    set_pool(&mut e, P0, &[(ManaColor::Blue, 1)]);
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: bomb,
                ability_index: 1
            }
        )
        .is_err()
    );
    set_pool(&mut e, P0, &[(ManaColor::Red, 1)]);
    activate_as(&mut e, P0, bomb, 1);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
}
