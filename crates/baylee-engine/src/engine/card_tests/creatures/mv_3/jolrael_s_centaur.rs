//! `cards/creatures/mv_3/jolrael_s_centaur.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CADAVEROUS_KNIGHT, GRIZZLY_BEARS, LIGHTNING_BOLT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jolrael_s_centaur() -> CardIndex {
    card_index("39b699e1-79c6-4960-97dc-d58574181bbe")
}

/// Jolrael's Centaur (39b699e1-79c6-4960-97dc-d58574181bbe) — {1}{G}{G} 2/2:
/// "Shroud, flanking."
#[test]
fn jolrael_s_centaur_has_shroud_and_flanking() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jolrael_s_centaur()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);

    let centaur = on_battlefield(&engine, P0, jolrael_s_centaur()).unwrap();
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
    assert_eq!(
        (chars.power, chars.toughness),
        (Some(1), Some(1)),
        "one -1/-1 on the 2/2, not two"
    );
}

/// "Shroud": a Lightning Bolt cannot be aimed at the Centaur, though the
/// opponent's Grizzly Bears is a legal target.
#[test]
fn jolrael_s_centaur_cannot_be_targeted() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[jolrael_s_centaur(), mountain()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let centaur = on_battlefield(&engine, P0, jolrael_s_centaur()).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };

    assert!(options.contains(&bears), "{options:?}");
    assert!(!options.contains(&centaur), "{options:?}");
}

/// "without flanking": a blocker with flanking (Cadaverous Knight) is not
/// shrunk.
#[test]
fn jolrael_s_centaur_does_not_shrink_a_flanking_blocker() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jolrael_s_centaur()])
        .battlefield(1, &[CADAVEROUS_KNIGHT])
        .start();
    keep_mulligans(&mut engine);

    let centaur = on_battlefield(&engine, P0, jolrael_s_centaur()).unwrap();
    let knight = on_battlefield(&engine, P1, CADAVEROUS_KNIGHT).unwrap();

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
                blockers: vec![(knight, centaur)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, knight), (2, 2));
}
