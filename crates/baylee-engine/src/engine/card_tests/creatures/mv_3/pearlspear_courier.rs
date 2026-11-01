//! `cards/creatures/mv_3/pearlspear_courier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Pearlspear Courier (`45c48bd8-9150-4699-a1eb-f4883165a4a0`) — {2}{W} Creature — Human Soldier 2/1:
/// "You may choose not to untap this creature during your untap step.
/// {2}{W}, {T}: Target Soldier creature gets +2/+2 and has vigilance for as long as this creature remains tapped."
#[test]
fn pearlspear_courier_grants_soldier_plus_two_plus_two_and_vigilance_while_tapped() {
    let courier_card = card_index("45c48bd8-9150-4699-a1eb-f4883165a4a0");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                courier_card,
                index::EAGER_CADET,
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let courier = on_battlefield(&engine, p0, courier_card).unwrap();
    let soldier = on_battlefield(&engine, p0, index::EAGER_CADET).unwrap();
    assert_eq!(pt(&engine, soldier), (1, 1));
    assert!(!keywords(&engine, soldier).contains(KeywordSet::VIGILANCE));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, courier_card, 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![soldier],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, soldier), (3, 3));
    assert!(keywords(&engine, soldier).contains(KeywordSet::VIGILANCE));

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
    assert_eq!(pt(&engine, soldier), (3, 3));
    assert!(keywords(&engine, soldier).contains(KeywordSet::VIGILANCE));

    // On the following turn, allow Pearlspear Courier to untap normally.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, courier));
    assert_eq!(pt(&engine, soldier), (1, 1));
    assert!(!keywords(&engine, soldier).contains(KeywordSet::VIGILANCE));
}
