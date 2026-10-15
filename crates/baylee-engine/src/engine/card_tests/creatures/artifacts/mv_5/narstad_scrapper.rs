//! `cards/creatures/artifacts/mv_5/narstad_scrapper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn narstad_scrapper() -> CardIndex {
    card_index("16acd858-dba6-401e-bbfb-cabdd6d0120e")
}

/// Narstad Scrapper — "{2}: This creature gets +1/+0 until end of turn." Each
/// payment adds another +1/+0, and it ends with the turn.
#[test]
fn narstad_scrapper_pumps_power_for_two_each_time() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[narstad_scrapper(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let scrapper = on_battlefield(&engine, p0, narstad_scrapper()).expect("seated");
    assert_eq!(pt(&engine, scrapper), (3, 3));

    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests[..2].contains(&id));
    activate(&mut engine, p0, narstad_scrapper(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, scrapper), (4, 3));

    tap_mana_where(&mut engine, p0, |id| forests[2..].contains(&id));
    activate(&mut engine, p0, narstad_scrapper(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, scrapper), (5, 3), "toughness is untouched");

    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert_eq!(pt(&engine, scrapper), (3, 3), "until end of turn");
}
