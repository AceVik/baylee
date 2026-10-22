//! `cards/artifacts/mv_2/null_rod.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{MOX_SAPPHIRE, NULL_ROD};

/// `Null Rod` ({2}, Artifact) prevents activated abilities of artifacts from
/// being activated.
#[test]
fn null_rod_prevents_artifact_activated_abilities() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[MOX_SAPPHIRE, island(), island()])
        .hand(0, &[NULL_ROD])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mox = on_battlefield(&engine, p0, MOX_SAPPHIRE).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(legal.abilities.contains(&(mox, 0)));

    tap_all_mana_but(&mut engine, p0, Some(MOX_SAPPHIRE));
    cast_with_floating(&mut engine, p0, NULL_ROD);
    pass_until(&mut engine, stack_is_empty);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(mox, 0)),
        "Null Rod stops artifact activated abilities including mana abilities"
    );
}
