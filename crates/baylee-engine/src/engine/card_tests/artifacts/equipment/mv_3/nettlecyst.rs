//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn living_weapon_chooses_one_doubled_germ_before_state_based_actions() {
    use crate::engine::card_tests::token_helpers::named;
    let p0 = PlayerId::new(0);
    for removed in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::DOUBLING_SEASON])
            .hand(0, &[index::NETTLECYST])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 3);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, index::NETTLECYST);
        pass_until(&mut e, |e| {
            on_battlefield(e, p0, index::NETTLECYST).is_some()
        });
        let equipment = on_battlefield(&e, p0, index::NETTLECYST).unwrap();
        if removed {
            bury(&mut e, &[equipment]);
            pass_until(&mut e, stack_is_empty);
            assert!(named(&e, "Phyrexian Germ").is_empty());
        } else {
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseCards { .. })
            });
            let Pending::ChooseCards {
                options, min, max, ..
            } = e.pending()
            else {
                unreachable!()
            };
            assert_eq!(options.len(), 2);
            assert_eq!((*min, *max), (1, 1));
            let chosen = options[1];
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![chosen],
                },
            )
            .unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(named(&e, "Phyrexian Germ"), vec![chosen]);
            assert_eq!(
                e.state().object(equipment).unwrap().attached_to,
                Some(chosen)
            );
            assert_eq!(pt(&e, chosen), (2, 2));
        }
    }
}

#[test]
fn an_animated_living_weapon_does_not_attach_while_it_is_a_creature() {
    use crate::engine::card_tests::token_helpers::named;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[index::NETTLECYST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, index::NETTLECYST);
    pass_until(&mut e, |e| {
        on_battlefield(e, p0, index::NETTLECYST).is_some()
    });
    let equipment = on_battlefield(&e, p0, index::NETTLECYST).unwrap();
    let chars = e
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(equipment)
        .unwrap()
        .base_mut();
    chars.types = chars.types.union(baylee_core::types::TypeSet::CREATURE);
    chars.power = Some(3);
    chars.toughness = Some(3);
    pass_until(&mut e, stack_is_empty);
    assert!(named(&e, "Phyrexian Germ").is_empty());
    assert_eq!(e.state().object(equipment).unwrap().attached_to, None);
}
