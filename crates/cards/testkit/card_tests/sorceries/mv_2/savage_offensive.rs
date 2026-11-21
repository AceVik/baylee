//! `cards/sorceries/mv_2/savage_offensive.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn savage_offensive() -> CardIndex {
    card_index("b5944ca8-d54e-4830-9f66-be8fe97d101d")
}

fn cast_savage(engine: &mut Engine<RegistryLookup>, kicked: bool) {
    cast_with_floating(engine, P0, savage_offensive());
    match engine.pending().clone() {
        Pending::YesNo { .. } => lk::yes_no(engine, kicked),
        Pending::ChooseCastMode { .. } => {
            let kind = if kicked {
                CastModeKind::Kicked
            } else {
                CastModeKind::Normal
            };
            let slot = choose_cast_kind(engine, kind);
            engine.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
        }
        _ => assert!(
            !kicked,
            "expected kicker prompt, got {:?}",
            engine.pending()
        ),
    }
    lk::settle(engine);
}

/// Savage Offensive (b5944ca8-d54e-4830-9f66-be8fe97d101d) — {1}{R} Sorcery,
/// Kicker {G}: "Creatures you control gain first strike until end of turn. If
/// this spell was kicked, they get +1/+1 until end of turn."
#[test]
fn savage_offensive_grants_first_strike_and_adds_one_one_when_kicked() {
    let mut unkicked = lk::game(
        SEED,
        &[mountain(), mountain(), forest(), aurochs()],
        &[aurochs()],
        &[savage_offensive()],
        &[],
    );
    let my_beast = lk::obj(&unkicked, P0, aurochs());
    let their_beast = lk::obj(&unkicked, P1, aurochs());

    tap_all_mana(&mut unkicked, P0);
    cast_savage(&mut unkicked, false);
    assert!(keywords(&unkicked, my_beast).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(pt(&unkicked, my_beast), (2, 3));
    assert!(!keywords(&unkicked, their_beast).contains(KeywordSet::FIRST_STRIKE));

    reach_their_main_phase(&mut unkicked, P1);
    assert!(!keywords(&unkicked, my_beast).contains(KeywordSet::FIRST_STRIKE));

    let mut kicked = lk::game(
        SEED,
        &[mountain(), mountain(), forest(), aurochs()],
        &[aurochs()],
        &[savage_offensive()],
        &[],
    );
    let my_kicked = lk::obj(&kicked, P0, aurochs());
    let their_kicked = lk::obj(&kicked, P1, aurochs());

    tap_all_mana(&mut kicked, P0);
    cast_savage(&mut kicked, true);
    assert_eq!(lk::pool_total(&kicked, P0), 0);
    assert!(keywords(&kicked, my_kicked).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(
        pt(&kicked, my_kicked),
        (3, 4),
        "both first strike and +1/+1"
    );
    assert_eq!(pt(&kicked, their_kicked), (2, 3));
}
