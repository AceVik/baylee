//! `cards/creatures/artifacts/mv_4/dross_ripper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dross_ripper() -> CardIndex {
    card_index("bf3b7489-e9d6-4b3c-9fb8-3bb727584db9")
}

/// Dross Ripper — {4} 3/3: "{2}{B}: This creature gets +1/+1 until end of
/// turn." Each payment adds one, the second stacks on the first (3/3, 4/4,
/// 5/5), and the next turn it is a 3/3 again.
#[test]
fn dross_ripper_firebreathes_a_plus_one_per_payment() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                dross_ripper(),
                swamp(),
                forest(),
                forest(),
                swamp(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ripper = on_battlefield(&engine, p0, dross_ripper()).expect("seated");
    assert_eq!(pt(&engine, ripper), (3, 3));

    // One Swamp and two Forests for the first payment, the rest for the
    // second, so that neither pays the other's {B} with its generic mana.
    let swamps = all_on_battlefield(&engine, p0, swamp());
    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| {
        id == swamps[0] || forests[..2].contains(&id)
    });
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    activate(&mut engine, p0, dross_ripper(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, ripper), (4, 4));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{2}}{{B}}");
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    activate(&mut engine, p0, dross_ripper(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, ripper), (5, 5), "the boosts stack");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert_eq!(pt(&engine, ripper), (3, 3), "until end of turn");
}
