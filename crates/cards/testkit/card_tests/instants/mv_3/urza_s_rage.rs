//! `cards/instants/mv_3/urza_s_rage.rs`, played with and without kicker.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "363f8c66-fe0c-44b9-987d-1d160e3f9c54"
fn urza_s_rage() -> CardIndex {
    card_index("363f8c66-fe0c-44b9-987d-1d160e3f9c54")
}

fn cast_rage(e: &mut Engine<RegistryLookup>, kicked: bool) {
    cast_with_floating(e, P0, urza_s_rage());
    if kicked {
        assert!(
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::Kicker,
                    ..
                }
            ),
            "kicker question: {:?}",
            e.pending()
        );
        e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    } else {
        assert!(
            matches!(e.pending(), Pending::ChooseTargets { .. }),
            "unkicked cast targets directly when kicker is unaffordable: {:?}",
            e.pending()
        );
    }
    lk::aim(e, &[], &[P1]);
}

fn their_priority(e: &mut Engine<RegistryLookup>) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
}

fn salve(e: &mut Engine<RegistryLookup>) {
    lk::float(e, P1, &[(ManaColor::White, 1)]);
    cast_with_floating(e, P1, index::HEALING_SALVE);
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        panic!("Salve mode")
    };
    let slot = options
        .iter()
        .position(|option| matches!(option.kind, CastModeKind::Mode(1)))
        .unwrap();
    e.apply(P1, PlayerAction::ChooseMode(slot)).unwrap();
    lk::aim(e, &[], &[P1]);
}

fn finish(e: &mut Engine<RegistryLookup>) {
    for _ in 0..50 {
        if stack_is_empty(e) {
            return;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseDamageEffect {
                player,
                choice,
                options,
                ..
            } => {
                assert_eq!(options.len(), 1);
                e.apply(
                    player,
                    PlayerAction::ChooseDamageEffect {
                        choice,
                        effect: options[0].id,
                    },
                )
                .unwrap();
            }
            Pending::AllocatePrevention {
                player,
                choice,
                damage,
                total,
                ..
            } => {
                assert_eq!(damage.len(), 1);
                e.apply(
                    player,
                    PlayerAction::AllocatePrevention {
                        choice,
                        allocation: vec![(damage[0].id, total)],
                    },
                )
                .unwrap();
            }
            pending => panic!("unexpected damage question: {pending:?}"),
        }
    }
    panic!("Urza's Rage did not finish");
}

/// Unkicked Rage costs {2}{R} and deals 3; paying its {8}{R} kicker instead
/// deals 10 to the same target and consumes the complete {10}{R}{R} cost.
#[test]
fn urzas_rage_deals_three_or_ten_when_kicked() {
    for (kicked, red_mana, expected_life) in [(false, 3, 17), (true, 12, 10)] {
        let mut e = lk::game(SEED, &[], &[], &[urza_s_rage()], &[]);
        lk::float(&mut e, P0, &[(ManaColor::Red, red_mana)]);
        cast_rage(&mut e, kicked);
        lk::settle(&mut e);
        assert_eq!(lk::life(&e, P1), expected_life);
        assert_eq!(lk::pool_total(&e, P0), 0);
        assert!(in_graveyard(&e, P0, urza_s_rage()).is_some());
    }
}

/// Healing Salve prevents all 3 ordinary damage, but the kicked spell's 10
/// damage remains unpreventable and is dealt in full through the same shield.
#[test]
fn urzas_rage_only_kicked_damage_cannot_be_prevented() {
    for (kicked, red_mana, expected_life) in [(false, 3, 20), (true, 12, 10)] {
        let mut e = lk::game(SEED, &[], &[], &[urza_s_rage()], &[index::HEALING_SALVE]);
        lk::float(&mut e, P0, &[(ManaColor::Red, red_mana)]);
        cast_rage(&mut e, kicked);
        their_priority(&mut e);
        salve(&mut e);
        finish(&mut e);
        assert_eq!(lk::life(&e, P1), expected_life);
    }
}

/// Counterspell can target Rage and resolves normally, but cannot remove it
/// from the stack; Rage then resolves and still deals its ordinary 3 damage.
#[test]
fn urzas_rage_cannot_be_countered() {
    let mut e = lk::game(SEED, &[], &[], &[urza_s_rage()], &[index::COUNTERSPELL]);
    lk::float(&mut e, P0, &[(ManaColor::Red, 3)]);
    cast_rage(&mut e, false);
    let rage = on_stack(&e, urza_s_rage()).unwrap();
    their_priority(&mut e);
    lk::float(&mut e, P1, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut e, P1, index::COUNTERSPELL);
    lk::aim(&mut e, &[rage], &[]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(lk::life(&e, P1), 17);
    assert!(in_graveyard(&e, P0, urza_s_rage()).is_some());
    assert!(in_graveyard(&e, P1, index::COUNTERSPELL).is_some());
}
