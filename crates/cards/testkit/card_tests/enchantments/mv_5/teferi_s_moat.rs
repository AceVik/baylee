//! `cards/enchantments/mv_5/teferi_s_moat.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Teferi's Moat — {3}{W}{U} Enchantment
/// "As this enchantment enters, choose a color.
/// Creatures of the chosen color without flying can't attack you."
#[test]
fn teferis_moat_stops_ground_creatures_of_chosen_color_from_attacking_you_while_fliers_and_other_colors_may_attack()
 {
    let moat = card_index("dafd319e-1d4a-4e08-9167-a82ad8670f6f");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), island()])
        .hand(0, &[moat])
        .battlefield(
            1,
            &[index::GRIZZLY_BEARS, index::GRAY_OGRE, index::AIR_ELEMENTAL],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, moat);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("choose Green as Teferi's Moat enters");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, moat).is_some());

    // Advance to P1's declare-attackers step.
    assert!(walk_to_own_main(&mut engine, p1));
    let green_ground = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let red_ground = on_battlefield(&engine, p1, index::GRAY_OGRE).unwrap();
    let blue_flier = on_battlefield(&engine, p1, index::AIR_ELEMENTAL).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending() else {
        panic!("expected ChooseAttackers for p1");
    };
    assert!(
        !attackers.contains(&green_ground),
        "green ground creature cannot attack p0 under Teferi's Moat naming green"
    );
    assert!(
        attackers.contains(&red_ground),
        "non-green ground creature can still attack p0"
    );
    assert!(
        attackers.contains(&blue_flier),
        "flying creature can still attack p0"
    );
}
