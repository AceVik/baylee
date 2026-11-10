//! `cards/creatures/mv_4/rappelling_scouts.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "c71eb9b5-2017-4e6c-8140-f9c8af65893f"
fn rappelling_scouts() -> CardIndex {
    card_index("c71eb9b5-2017-4e6c-8140-f9c8af65893f")
}

/// `Rappelling Scouts` (`Coverage::Implemented`):
/// "Flying. {2}{W}: This creature gains protection from the color of your choice
/// until end of turn."
///
/// Verifies that `Rappelling Scouts` has flying and that its `{2}{W}` activated
/// ability grants it protection from the chosen color until end of turn
/// (preventing a spell of that color from targeting it) and wears off at cleanup.
#[test]
fn rappelling_scouts_gains_protection_from_chosen_color_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9513, plains())
        .battlefield(
            0,
            &[
                rappelling_scouts(),
                grizzly_bears(),
                plains(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[index::DARK_BANISHING, index::DARK_BANISHING])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let scouts = on_battlefield(&engine, p0, rappelling_scouts()).expect("P0 Rappelling Scouts");
    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("P0 Grizzly Bears");
    assert!(keywords(&engine, scouts).contains(KeywordSet::FLYING));

    // Activate `{2}{W}: This creature gains protection from the color of your choice until end of turn.`
    let plains_ids = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| plains_ids.contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: scouts,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Cast Dark Banishing -> Rappelling Scouts cannot be targeted, Grizzly Bears can.
    let swamp_ids = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| swamp_ids.contains(&id));
    cast_with_floating(&mut engine, p0, index::DARK_BANISHING);
    let menu = aim_at(&mut engine, p0, bear);
    assert!(
        !menu.contains(&scouts),
        "protection from black prevents Dark Banishing from targeting Rappelling Scouts"
    );
    pass_until(&mut engine, stack_is_empty);

    // On P0's next turn, protection from black has worn off, so Dark Banishing can target Rappelling Scouts.
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    tap_mana_where(&mut engine, p0, |id| swamp_ids.contains(&id));
    cast_with_floating(&mut engine, p0, index::DARK_BANISHING);
    let menu = aim_at(&mut engine, p0, scouts);
    assert!(
        menu.contains(&scouts),
        "protection from black wore off at cleanup, allowing Dark Banishing to target Rappelling Scouts"
    );
}
