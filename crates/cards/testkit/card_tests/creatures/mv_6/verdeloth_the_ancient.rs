//! `cards/creatures/mv_6/verdeloth_the_ancient.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn verdeloth_the_ancient() -> CardIndex {
    card_index("8222321f-b74d-4f74-9ee7-79455433345a")
}

fn treefolk_harbinger() -> CardIndex {
    card_index("98c387bd-c9ba-464a-8380-839a5604a37f")
}

/// Verdeloth the Ancient ({4}{G}{G} 4/7 Legendary Creature — Treefolk, Kicker {X}):
/// - Saproling creatures and other Treefolk creatures get +1/+1 (not Verdeloth itself).
/// - When Verdeloth enters, if it was kicked, create X 1/1 green Saproling creature tokens (which become 2/2).
#[test]
fn verdeloth_pumps_saprolings_and_other_treefolk_and_creates_kicked_x_saprolings() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                treefolk_harbinger(),
            ],
        )
        .hand(0, &[verdeloth_the_ancient()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let harbinger = on_battlefield(&engine, P0, treefolk_harbinger()).unwrap();
    assert_eq!(pt(&engine, harbinger), (0, 3));

    // Cast kicked with X = 3 ({4}{G}{G} + {3} = 9 mana).
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, verdeloth_the_ancient());
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let verdeloth = on_battlefield(&engine, P0, verdeloth_the_ancient()).unwrap();
    assert_eq!(
        pt(&engine, verdeloth),
        (4, 7),
        "Verdeloth does not pump itself"
    );
    assert_eq!(pt(&engine, harbinger), (1, 4), "other Treefolk gets +1/+1");

    let saprolings: Vec<_> = engine
        .state()
        .battlefield_seen()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.card.is_none()
                    && engine.state().names.get(o.characteristics().name) == "Saproling"
            })
        })
        .collect();
    assert_eq!(saprolings.len(), 3);
    assert_eq!(
        pt(&engine, saprolings[0]),
        (2, 2),
        "Saproling 1/1 buffed to 2/2"
    );
}
