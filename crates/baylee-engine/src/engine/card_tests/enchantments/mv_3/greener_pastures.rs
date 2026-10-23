#[allow(clippy::wildcard_imports)]
use super::*;

fn greener_pastures() -> CardIndex {
    card_index("f9e1346a-af72-4644-b384-8f7b0a35ec31")
}

#[test]
fn greener_pastures_creates_saproling_token_on_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[greener_pastures()])
        .start();
    keep_mulligans(&mut engine);

    assert!(walk_to_own_main(&mut engine, p0));
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .len(),
        2
    );
}
