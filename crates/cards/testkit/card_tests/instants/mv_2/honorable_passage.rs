//! `cards/instants/mv_2/honorable_passage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn honorable_passage() -> CardIndex {
    card_index("66805334-1015-4b41-ba9b-116de94b0744")
}

/// P0 casts Honorable Passage in the opponent's first main phase, naming `source_card` of P1's as
/// the source, and the opponent then attacks with `attack_with`; returns the engine after combat.
#[track_caller]
fn passage_then_attack(
    source_card: CardIndex,
    attack_with: &[CardIndex],
) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[honorable_passage()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let source_object = on_battlefield(&engine, p1, source_card).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, honorable_passage());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    let source = options
        .into_iter()
        .find(|s| s.object == source_object)
        .expect("the creature is a damage source option");
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let attackers = attack_with
        .iter()
        .map(|card| {
            (
                on_battlefield(&engine, p1, *card).unwrap(),
                Defender::Player(p0),
            )
        })
        .collect();
    engine
        .apply(p1, PlayerAction::DeclareAttackers { attackers })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    engine
}

/// `Honorable Passage` ({1}{W}, Instant, `Coverage::Partial`) prevents the next damage a chosen
/// source would deal to you this turn. The chosen Bears' 2 damage is prevented, the other
/// attacker's is not.
#[test]
fn honorable_passage_prevents_only_the_chosen_sources_damage_to_you() {
    let engine = passage_then_attack(grizzly_bears(), &[grizzly_bears(), gray_ogre()]);
    assert_eq!(
        engine.state().players[0].life,
        18,
        "the Bears' damage is prevented, the Ogre's 2 is not"
    );
}

/// The chosen source is a red creature: its damage is prevented, but the printed "deals that much
/// damage to the source's controller" is not played (Partial), so the opponent stays at 20.
#[test]
fn honorable_passage_prevents_a_red_sources_damage_but_does_not_reflect_it_yet() {
    let engine = passage_then_attack(gray_ogre(), &[gray_ogre()]);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the Ogre's damage is prevented"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "Partial: the reflected damage to the red source's controller is not played"
    );
}
