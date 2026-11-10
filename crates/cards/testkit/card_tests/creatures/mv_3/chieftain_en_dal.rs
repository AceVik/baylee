//! `cards/creatures/mv_3/chieftain_en_dal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Chieftain en-Dal ({1}{W}{W} 2/2 Creature — Human Knight):
/// "Whenever this creature attacks, attacking creatures gain first strike until end of turn."
#[test]
fn chieftain_en_dal_grants_attacking_creatures_first_strike_when_it_attacks() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                card_index("8aae3a28-dcf3-460e-89f9-4b0c96d6f55e"),
                grizzly_bears(),
                index::HILL_GIANT,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let chieftain = on_battlefield(
        &engine,
        P0,
        card_index("8aae3a28-dcf3-460e-89f9-4b0c96d6f55e"),
    )
    .unwrap();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (chieftain, Defender::Player(P1)),
                    (bears, Defender::Player(P1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(chieftain)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );
    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );
    assert!(
        !engine
            .state()
            .object(giant)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE),
        "non-attacking Hill Giant does not gain first strike"
    );
}
