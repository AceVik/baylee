//! Oracle scenarios for Angelic Edict.
#[allow(clippy::wildcard_imports)]
use super::*;
fn angelic_edict() -> CardIndex {
    card_index("36de8401-ee4e-413f-91ad-06924e39c857")
}

fn in_exile(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|object| object.card.is_some_and(|printing| printing.index == card))
        })
}

// Checklist: target may be a creature or enchantment; exile is the destination.
#[test]
fn angelic_edict_exiles_its_creature_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[angelic_edict(), angelic_edict()])
        .battlefield(1, &[wild_elephant(), their_enchantment()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let creature = on_battlefield(&e, p1, wild_elephant()).unwrap();
    let enchantment = on_battlefield(&e, p1, their_enchantment()).unwrap();
    cast_from_hand(&mut e, p0, angelic_edict());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("creature or enchantment target")
    };
    assert!(options.contains(&creature) && options.contains(&enchantment));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![creature],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_exile(&e, p1, wild_elephant()));
    assert!(on_battlefield(&e, p1, their_enchantment()).is_some());

    cast_from_hand(&mut e, p0, angelic_edict());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("creature or enchantment target")
    };
    assert!(options.contains(&enchantment));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![enchantment],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_exile(&e, p1, their_enchantment()));
}
