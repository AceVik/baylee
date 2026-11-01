//! `cards/creatures/mv_1/mtenda_herder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::KeywordSet;
use baylee_core::generated::index;
use baylee_core::ids::Defender;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "0edf5bac-36a9-4f35-afd1-73f7e1aab51b"
fn mtenda_herder() -> CardIndex {
    card_index("0edf5bac-36a9-4f35-afd1-73f7e1aab51b")
}

#[test]
fn mtenda_herder_has_flanking_and_shrinks_non_flanking_blocker() {
    let mut engine = lk::game(SEED, &[mtenda_herder()], &[index::GRIZZLY_BEARS], &[], &[]);
    let herder = lk::obj(&engine, P0, mtenda_herder());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    assert!(
        engine
            .state()
            .object(herder)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLANKING)
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(herder, Defender::Player(P1))],
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
                blockers: vec![(bears, herder)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    // Grizzly Bears (2/2) got -1/-1 from flanking (to 1/1) and took 1 combat damage from Mtenda Herder, so both died.
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
}

/// Flanking only shrinks a blocker "without flanking": a Knight of Valor
/// (2/2, flanking itself) blocks Mtenda Herder untouched, survives the 1
/// damage and kills the 1/1 Herder.
#[test]
fn mtenda_herder_does_not_shrink_a_blocker_that_has_flanking() {
    let mut engine = lk::game(
        SEED,
        &[mtenda_herder()],
        &[index::KNIGHT_OF_VALOR],
        &[],
        &[],
    );
    let herder = lk::obj(&engine, P0, mtenda_herder());
    let knight = lk::obj(&engine, P1, index::KNIGHT_OF_VALOR);

    let offer = lk::attack(&mut engine, &[herder]);
    assert!(lk::may_block(&offer, knight, herder));
    lk::block(&mut engine, &[(knight, herder)]);
    lk::through_combat(&mut engine);

    assert!(
        on_battlefield(&engine, P1, index::KNIGHT_OF_VALOR).is_some(),
        "a blocker with flanking gets no -1/-1, so 1 damage does not kill a 2/2"
    );
    assert!(
        on_battlefield(&engine, P0, mtenda_herder()).is_none(),
        "the Knight deals 2 damage to the 1/1 Herder"
    );
}
