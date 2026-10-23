#[allow(clippy::wildcard_imports)]
use super::*;

fn metrognome() -> CardIndex {
    card_index("57fe5c94-ee93-45c6-bf7e-89d177d0fe70")
}

#[test]
fn metrognome_creates_gnome_token_when_activated() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[metrognome(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, metrognome(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .len(),
        before + 1
    );
}
