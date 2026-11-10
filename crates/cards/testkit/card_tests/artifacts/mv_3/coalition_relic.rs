//! `cards/artifacts/mv_3/coalition_relic.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn coalition_relic() -> CardIndex {
    card_index("008cb342-79f5-4df6-a6b7-0e9e22ed693f")
}

/// Resolves every Relic trigger already waiting at first main and counts the
/// colour questions: an empty Relic asks none and adds nothing, a charged
/// one asks once per counter removed.
fn drain_first_main_triggers(
    e: &mut Engine<RegistryLookup>,
    player: PlayerId,
    color: ManaColor,
) -> usize {
    let mut choices = 0;
    for _ in 0..20 {
        if stack_is_empty(e) {
            return choices;
        }
        match e.pending().clone() {
            Pending::ChooseColor { .. } => {
                e.apply(player, PlayerAction::ChooseColor(color))
                    .expect("the trigger's color choice is legal");
                choices += 1;
            }
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority)
                    .expect("priority advances the waiting trigger");
            }
            other => panic!("unexpected first-main question: {other:?}"),
        }
    }
    panic!("the first-main triggers did not drain");
}

/// Both tap abilities are played. The stored charge is removed at the next
/// own first main phase and becomes exactly one mana of the chosen color.
#[test]
fn coalition_relic_makes_immediate_mana_then_releases_its_stored_charge() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[coalition_relic(), coalition_relic()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    drain_first_main_triggers(&mut e, p0, ManaColor::White);

    activate(&mut e, p0, coalition_relic(), 0);
    let Pending::ChooseColor { options, .. } = e.pending().clone() else {
        panic!("expected any-color mana, got {:?}", e.pending())
    };
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(options.contains(&color), "missing {color:?}");
    }
    assert_eq!(options.len(), 5);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("any color includes red");
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Red), 1);

    activate(&mut e, p0, coalition_relic(), 1);
    pass_until(&mut e, stack_is_empty);
    let relics = all_on_battlefield(&e, p0, coalition_relic());
    let charged = relics
        .iter()
        .copied()
        .find(|id| counters_on(&e, *id, CounterKind::Charge) == 1)
        .expect("the second tap ability put on one charge counter");

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    let blue_before = e.state().players[0].mana_pool.available(ManaColor::Blue);
    assert_eq!(
        drain_first_main_triggers(&mut e, p0, ManaColor::Blue),
        1,
        "the charged Relic asks, the empty one does not"
    );
    assert_eq!(counters_on(&e, charged, CounterKind::Charge), 0);
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Blue),
        blue_before + 1,
        "only the one removed counter produced mana"
    );
}
