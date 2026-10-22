//! `cards/artifacts/mv_2/jabari_s_banner.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRAY_OGRE, GRIZZLY_BEARS, JABARI_S_BANNER};

/// `Jabari's Banner` ({2}, Artifact) — `{1}, {T}`: Target creature gains
/// flanking until end of turn.
#[test]
fn jabari_s_banner_grants_flanking_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[JABARI_S_BANNER, GRIZZLY_BEARS, plains()])
        .battlefield(1, &[GRAY_OGRE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&engine, p1, GRAY_OGRE).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, JABARI_S_BANNER, 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ogre), (1, 1));
}
