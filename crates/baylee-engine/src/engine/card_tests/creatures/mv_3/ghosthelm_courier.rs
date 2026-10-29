//! `cards/creatures/mv_3/ghosthelm_courier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Ghosthelm Courier (`e447f7cc-4cdc-4a79-8058-8c1f1224d357`) — {2}{U} Creature — Human Wizard 2/1:
/// "You may choose not to untap this creature during your untap step.
/// {2}{U}, {T}: Target Wizard creature gets +2/+2 and has shroud for as long as this creature remains tapped."
#[test]
fn ghosthelm_courier_grants_wizard_plus_two_plus_two_and_shroud_while_tapped() {
    let courier_card = card_index("e447f7cc-4cdc-4a79-8058-8c1f1224d357");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                courier_card,
                index::PRODIGAL_SORCERER,
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let courier = on_battlefield(&engine, p0, courier_card).unwrap();
    let wizard = on_battlefield(&engine, p0, index::PRODIGAL_SORCERER).unwrap();
    assert_eq!(pt(&engine, wizard), (1, 1));
    assert!(!keywords(&engine, wizard).contains(KeywordSet::SHROUD));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, courier_card, 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wizard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, wizard), (3, 3));
    assert!(keywords(&engine, wizard).contains(KeywordSet::SHROUD));

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![courier],
            },
        )
        .unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, courier));
    assert_eq!(pt(&engine, wizard), (3, 3));
    assert!(keywords(&engine, wizard).contains(KeywordSet::SHROUD));

    // On the following turn, allow Ghosthelm Courier to untap normally.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, courier));
    assert_eq!(pt(&engine, wizard), (1, 1));
    assert!(!keywords(&engine, wizard).contains(KeywordSet::SHROUD));
}
