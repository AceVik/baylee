//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn borrower_blocks_flying_but_not_reach_or_ground_creatures() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::BRAZEN_BORROWER])
        .battlefield(1, &[index::ORNITHOPTER, grizzly_bears(), giant_spider()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let b = on_battlefield(&e, p0, index::BRAZEN_BORROWER).unwrap();
    for (card, allowed) in [
        (index::ORNITHOPTER, true),
        (grizzly_bears(), false),
        (giant_spider(), false),
    ] {
        let a = on_battlefield(&e, p1, card).unwrap();
        assert_eq!(crate::combat::can_block(e.state(), p0, b, a), allowed);
    }
    assert!(keywords(&e, b).contains(KeywordSet::FLASH.union(KeywordSet::FLYING)));
}

#[test]
fn petty_theft_targets_as_an_instant_in_both_offer_and_target_choice() {
    use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use baylee_cards_dsl::{Duration, Filter, Modifier};
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (protection, legal) in [
        (&Filter::CREATURE, true),
        (
            &Filter::HasType(baylee_core::types::TypeSet::INSTANT),
            false,
        ),
    ] {
        let mut e = Duel::new(SEED, forest())
            .hand(0, &[brazen_borrower()])
            .battlefield(1, &[grizzly_bears()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bear = on_battlefield(&e, p1, grizzly_bears()).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        let filter = EffectFilter::object(state, bear);
        let modifier = Modifier::ProtectionFrom(protection);
        state.effects.register(ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(bear),
            controller: p1,
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::Indefinitely,
            filter,
            modifier,
        });
        give_mana(&mut e, p0, ManaColor::Blue, 3);
        let card = in_hand(&e, p0, brazen_borrower()).unwrap();
        assert_eq!(
            crate::casting::face_has_a_legal_target(e.state(), &RegistryLookup, p0, card, 1),
            legal
        );
        if !legal {
            continue;
        }
        e.apply(p0, PlayerAction::CastSpell { card }).unwrap();
        let Pending::ChooseCastMode { options, .. } = e.pending() else {
            panic!("face choice")
        };
        let mode = options
            .iter()
            .position(|o| o.kind == CastModeKind::Face(1))
            .unwrap();
        e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
        assert!(aim_at(&mut e, p0, bear).contains(&bear));
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(bear).unwrap().zone, Zone::Hand);
    }
}
