//! `cards/creatures/mv_3/jolrael_s_centaur.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, JOLRAEL_S_CENTAUR};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Jolrael's Centaur (31588022-31d2-412c-8474-782460506024) — {1}{G}{G} 2/2:
/// "Shroud, flanking."
#[test]
fn jolrael_s_centaur_has_shroud_and_flanking() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[JOLRAEL_S_CENTAUR])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);

    let centaur = on_battlefield(&engine, P0, JOLRAEL_S_CENTAUR).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let kw = engine
        .state()
        .object(centaur)
        .unwrap()
        .characteristics()
        .keywords;
    assert!(kw.contains(KeywordSet::SHROUD));
    assert!(kw.contains(KeywordSet::FLANKING));

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(centaur, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, centaur)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let chars = engine.state().object(bears).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(1), Some(1)));
}
