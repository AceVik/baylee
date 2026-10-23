#[allow(clippy::wildcard_imports)]
use super::*;

fn serra_avatar() -> CardIndex {
    card_index("c363725f-15cf-4955-aaf1-536b1f8daaed")
}

#[test]
fn serra_avatar_shuffles_into_library_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[serra_avatar()])
        .start();
    let avatar_id = on_battlefield(&engine, p0, serra_avatar()).expect("avatar on bf");
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(avatar_id).unwrap().zone,
        crate::zone::Zone::Library
    );
}
