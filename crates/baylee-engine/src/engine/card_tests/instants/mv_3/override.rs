//! Override counters unless the target spell's controller pays the artifact count.

use super::*;

fn override_card() -> CardIndex {
    card_index("a2845761-7e43-4a88-8fc2-2d95d471b00c")
}

fn respond_to_spell(pay: bool) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (their_lands, their_spell) = if pay {
        (5, gray_ogre())
    } else {
        (1, quiet_artifact())
    };
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                override_card(),
                quiet_artifact(),
                quiet_artifact(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &vec![mountain(); their_lands])
        .hand(0, &[override_card()])
        .hand(1, &[their_spell])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    cast_from_hand(&mut e, p1, their_spell);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let spell = *e.state().zones.list(ZoneLocation::Stack).last().unwrap();
    cast_from_hand(&mut e, p0, override_card());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![spell],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    e.apply(p1, PlayerAction::YesNo(pay)).unwrap();
    pass_until(&mut e, stack_is_empty);
    e
}

#[test]
fn override_counters_without_payment_and_allows_the_spell_after_payment() {
    let p1 = PlayerId::new(1);
    let e = respond_to_spell(false);
    assert!(in_graveyard(&e, p1, quiet_artifact()).is_some());
    assert!(on_battlefield(&e, p1, quiet_artifact()).is_none());

    let e = respond_to_spell(true);
    assert!(on_battlefield(&e, p1, gray_ogre()).is_some());
    assert_eq!(e.state().players[1].mana_pool.total(), 0);
}
