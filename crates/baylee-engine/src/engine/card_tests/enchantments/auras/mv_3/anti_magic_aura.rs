//! Printed rules exercised through real choices and resolution.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn anti_magic_aura_stops_spells_but_allows_activated_abilities() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                llanowar_elves(),
                island(),
                island(),
                island(),
                plains(),
                index::ICY_MANIPULATOR,
            ],
        )
        .hand(
            0,
            &[
                index::ANTI_MAGIC_AURA,
                swords_to_plowshares(),
                index::HOLY_STRENGTH,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    cast_from_hand(&mut e, p0, index::ANTI_MAGIC_AURA);
    aim_at(&mut e, p0, elf);
    pass_until(&mut e, stack_is_empty);
    let swords = in_hand(&e, p0, swords_to_plowshares()).unwrap();
    let aura = in_hand(&e, p0, index::HOLY_STRENGTH).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        !legal.castable.contains(&swords),
        "only creature refuses spells even at the offer stage"
    );
    assert!(!legal.castable.contains(&aura));
    assert!(
        crate::eval::spell_target_options_for(
            &baylee_cards_dsl::TargetSpec::Object(&baylee_cards_dsl::Filter::CREATURE),
            e.state(),
            p0,
            swords,
            None,
            None,
            false,
        )
        .is_empty()
    );
    e.dev_state_mut(p0).unwrap().set_tapped(elf, false);
    e.refresh_offer();
    activate(&mut e, p0, index::ICY_MANIPULATOR, 0);
    assert!(aim_at(&mut e, p0, elf).contains(&elf));
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, elf));
}
