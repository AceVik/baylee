//! `cards/creatures/mv_3/tireless_provisioner.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Tireless Provisioner` prints `Landfall — Whenever a land you control enters, create a Food token or a Treasure token. (Food is an artifact with "{{2}}, {{T}}, Sacrifice this token: You gain 3 life." Treasure is an artifact with "{{T}}, Sacrifice this token: Add one mana of any color.")`
///
/// Marked `Coverage::Implemented`, playing a land triggers `Trigger::EntersBattlefield` with `Filter::YOUR_LAND`, presenting a modal choice via `Pending::ChooseCastMode`.
/// Choosing mode 1 creates an artifact Treasure token verified by `tokens_of`.
#[test]
fn tireless_provisioner_creates_treasure_token_on_landfall() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tireless_provisioner()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let prov = on_battlefield(&engine, p0, tireless_provisioner()).expect("provisioner deployed");
    assert_eq!(pt(&engine, prov), (3, 2));
    assert!(tokens_of(&engine, p0).is_empty());

    play_land(&mut engine, p0, forest());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode prompt, got {:?}", engine.pending());
    };
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![0, 1],
        "offers Food (mode 0) and Treasure (mode 1)"
    );

    let treasure_slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("treasure mode");
    engine
        .apply(p0, PlayerAction::ChooseMode(treasure_slot))
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one token created");
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::ARTIFACT),
        "token has artifact type"
    );
}
