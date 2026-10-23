#[allow(clippy::wildcard_imports)]
use super::*;

fn rewind() -> CardIndex {
    card_index("bb27bfdf-fe8d-45bd-ad62-8118dce06eda")
}

#[test]
fn rewind_counters_target_spell_and_untaps_up_to_four_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[gray_ogre(), rewind()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, gray_ogre());
    let spell_id = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .last()
        .copied()
        .expect("spell on stack");

    cast_from_hand(&mut engine, p0, rewind());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, spell_id);
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: options.into_iter().take(4).collect(),
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p0, gray_ogre()).is_some());
    let untapped = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| !is_tapped(&engine, **id))
        .count();
    assert_eq!(untapped, 4, "only the chosen four lands untap");
}
