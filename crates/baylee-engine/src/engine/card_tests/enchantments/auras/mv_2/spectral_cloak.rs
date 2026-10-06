//! `cards/enchantments/auras/mv_2/spectral_cloak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Spectral Cloak` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature has shroud as long as it's untapped."
///
/// Verifies that an untapped creature enchanted by `Spectral Cloak` gains shroud,
/// while an opponent's creature is unaffected. When the enchanted creature taps to
/// produce mana, shroud is immediately lost because the static condition is no longer met.
#[test]
fn spectral_cloak_grants_shroud_only_while_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1301, forest())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[spectral_cloak()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elves deployed");

    // The two Islands and *not* the Elves: `cast_from_hand` taps every mana
    // source on the board, and the Elves are one — so paying that way would
    // tap the very creature whose untapped status this card reads, and the
    // missing shroud would look like an unimplemented static ability rather
    // than a correctly-read condition.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, spectral_cloak());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&mine) && options.contains(&theirs));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, spectral_cloak()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen creature"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::SHROUD),
        "untapped enchanted creature has shroud"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::SHROUD),
        "opponent creature has no shroud"
    );

    // Tapping the elf causes it to lose shroud
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mine,
                ability_index: 0,
            },
        )
        .expect("elf taps for mana");
    assert!(is_tapped(&engine, mine), "elf is now tapped");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::SHROUD),
        "shroud is lost while tapped"
    );
}
