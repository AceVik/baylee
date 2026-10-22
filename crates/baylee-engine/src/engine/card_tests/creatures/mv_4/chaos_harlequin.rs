//! `cards/creatures/mv_4/chaos_harlequin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn chaos_harlequin() -> CardIndex {
    card_index("c8e96a51-24c9-4955-8bf0-6ef448fa8011")
}

/// `Chaos Harlequin` ({2}{R}{R}, 2/4 Creature — Human, `Coverage::Partial`)
/// activates for `{R}` to get +2/+0 until end of turn.
#[test]
fn chaos_harlequin_pumps_power_by_two_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 6])
        .hand(0, &[chaos_harlequin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, chaos_harlequin());
    pass_until(&mut engine, stack_is_empty);
    let harlequin = on_battlefield(&engine, p0, chaos_harlequin()).expect("Chaos Harlequin enters");
    assert_eq!(pt(&engine, harlequin), (2, 4));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: harlequin,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, harlequin), (4, 4));

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: harlequin,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, harlequin), (6, 4));
}
