//! A departed replacement chooser must not end somebody else's resolution.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch47_regeneration_concession_preserves_other_choices_and_remaining_effects() {
    let p2 = PlayerId::new(2);
    let mut e = Duel::table(SEED, index::FOREST, 3)
        .hand(0, &[index::VINDICATE])
        .battlefield(1, &[index::CLERGY_OF_THE_HOLY_NIMBUS])
        .battlefield(2, &[index::CLERGY_OF_THE_HOLY_NIMBUS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let first = on_battlefield(&e, P1, index::CLERGY_OF_THE_HOLY_NIMBUS).unwrap();
    let second = on_battlefield(&e, p2, index::CLERGY_OF_THE_HOLY_NIMBUS).unwrap();
    for clergy in [first, second] {
        e.state.object_mut(clergy).unwrap().regeneration_shields = 1;
    }
    set_pool(
        &mut e,
        P0,
        &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
        ],
    );
    let spell = in_hand(&e, P0, index::VINDICATE).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    targets(&mut e, vec![first], vec![]);
    pass_until(&mut e, |engine| {
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::UseRegenerationShield { .. },
                ..
            }
        )
    });
    assert_eq!(e.pending().asked(), Some(P1));
    // Isolate a second destruction in the same decision-free instruction,
    // followed by another instruction. The actual Vindicate establishes the
    // resolution and the actual Clergy establish both replacement choices.
    crate::sba::destroy(&mut e.state, second);
    let life = e.state.players[0].life;
    e.resolution
        .as_mut()
        .expect("Vindicate suspended")
        .effects
        .push(baylee_cards_dsl::Effect::GainLife {
            amount: baylee_cards_dsl::Amount::Fixed(5),
        });
    assert_eq!(e.state.regeneration_choices.len(), 2);
    e.apply(P1, PlayerAction::Concede).unwrap();
    assert!(e.state.has_left(P1));
    assert_eq!(e.state.players[0].life, life, "the next chooser acts first");
    assert!(matches!(
        e.pending(),
        Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::UseRegenerationShield { permanent },
            ..
        } if *player == p2 && *permanent == second
    ));
    assert!(
        e.resolution.is_some(),
        "the surviving controller's resolution remains"
    );
    e.apply(p2, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life + 5);
    let survivor = e.state.object(second).unwrap();
    assert_eq!(survivor.zone, Zone::Battlefield);
    assert_eq!(survivor.regeneration_shields, 0);
    assert!(is_tapped(&e, second));
    assert!(e.state.regeneration_choices.is_empty());
}
