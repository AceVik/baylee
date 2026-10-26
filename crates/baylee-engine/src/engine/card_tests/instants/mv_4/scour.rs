//! `cards/instants/mv_4/scour.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn scour() -> CardIndex {
    card_index("f18622da-81f3-4854-a1bb-8d658751d8db")
}

/// Scour (f18622da-81f3-4854-a1bb-8d658751d8db) — {2}{W}{W} Instant (Partial):
/// "Exile target enchantment. Search its controller's graveyard, hand, and
/// library for all cards with the same name as that enchantment and exile
/// them. Then that player shuffles."
#[test]
fn scour_exiles_target_enchantment_and_excludes_non_enchantments() {
    use baylee_core::generated::index;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[index::GLORIOUS_ANTHEM, grizzly_bears(), sol_ring()])
        .hand(0, &[scour()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let anthem = on_battlefield(&engine, p1, index::GLORIOUS_ANTHEM).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let land = on_battlefield(&engine, p0, plains()).unwrap();

    cast_from_hand(&mut engine, p0, scour());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Scour");
    };
    assert!(options.contains(&anthem), "enchantment is a legal target");
    assert!(
        !options.contains(&bears) && !options.contains(&ring) && !options.contains(&land),
        "creatures, artifacts, and lands are not legal targets"
    );

    aim_at(&mut engine, p0, anthem);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::GLORIOUS_ANTHEM).is_none());
    assert_eq!(engine.state().object(anthem).unwrap().zone, Zone::Exile);
}
