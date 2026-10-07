//! `cards/lands/artifacts/ancient_den.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ancient Den prints no basic land type — its type line is `Artifact Land`
/// and its whole rules text is `{T}: Add {W}` — so the permanent has to be
/// played to exist, has to be an artifact *and* a land once it is, and has to
/// make its mana through a printed mana ability (CR 605.1) rather than the
/// CR 305.6 shortcut a basic land type would hand it. That last reading is
/// what the offer is asked for: the route is an ordinary `(source, index)`
/// entry in `abilities` and never a bare source in `mana_abilities`. The
/// board holds nothing else, so one white mana in an otherwise empty pool,
/// off a tapped Den on an empty stack (CR 605.3b), is the whole of what the
/// card does.
#[test]
fn ancient_den_arrives_as_an_artifact_land_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[ancient_den()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let den = play_land(&mut engine, p0, ancient_den());
    assert!(
        !is_tapped(&engine, den),
        "a land with no enter modifier arrives untapped, which is what lets \
         it pay for something in the turn it lands on"
    );

    let kinds = types(&engine, den);
    assert!(
        kinds.contains(TypeSet::LAND) && kinds.contains(TypeSet::ARTIFACT),
        "`Artifact Land` is both, and the second half is the one every \
         artifact-counting effect on the table reads: {kinds:?}"
    );

    // The offer before the tap, because that is the roof `tap_all_mana`
    // works under: it can only press what the engine has already listed.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(den, 0)),
        "the printed `{{T}}: Add {{W}}` is an ordinary (source, index) entry \
         in `abilities`: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&den),
        "and not the CR 305.6 shortcut in `mana_abilities`: the Den carries no \
         basic land type for `intrinsic_mana` to read"
    );

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, den), "the Den paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "one white mana, and the battlefield holds no other source that \
         could have made it"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
}
