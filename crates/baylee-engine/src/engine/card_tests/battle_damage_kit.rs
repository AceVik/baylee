//! Actual Siege entry for the any-target cards whose Battle coverage was held.
#[allow(clippy::wildcard_imports)] // The card tests' shared harness vocabulary.
use super::*;

pub(super) fn belenon() -> CardIndex {
    card_index("e462e78d-2d26-4a2d-8d8f-5f31abe5c924")
}

pub(super) fn game(
    seed: u64,
    board: &[CardIndex],
    hand: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut hand = hand.to_vec();
    hand.push(belenon());
    let mut e = Duel::new(seed, forest())
        .battlefield(0, board)
        .battlefield(1, &[fangren_hunter(), quiet_artifact()])
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    float(&mut e, ManaColor::White, 3);
    cast_with_floating(&mut e, p0, belenon());
    pass_until(&mut e, |e| {
        on_battlefield(e, p0, belenon()).is_some()
            && e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { .. })
            && knight(e).is_some()
    });
    let battle = on_battlefield(&e, p0, belenon()).unwrap();
    assert_eq!(
        defense(&e, battle),
        5,
        "printed defense placed by actual entry"
    );
    assert_eq!(protector(&e, battle), PlayerId::new(1));
    assert_eq!(
        pt(&e, knight(&e).unwrap()),
        (2, 2),
        "the front's ETB resolved"
    );
    (e, battle)
}

pub(super) fn float(e: &mut Engine<RegistryLookup>, color: ManaColor, amount: u32) {
    e.dev_state_mut(PlayerId::new(0)).unwrap().players[0]
        .mana_pool
        .add(color, amount);
    e.refresh_offer();
}

pub(super) fn defense(e: &Engine<RegistryLookup>, battle: ObjectId) -> u16 {
    e.state()
        .object(battle)
        .unwrap()
        .counters
        .get(baylee_cards_dsl::CounterKind::Defense)
}

pub(super) fn protector(e: &Engine<RegistryLookup>, battle: ObjectId) -> PlayerId {
    e.state()
        .object(battle)
        .unwrap()
        .riders
        .iter()
        .find_map(|rider| match rider {
            crate::object::Rider::BattleProtector(player) => Some(*player),
            _ => None,
        })
        .expect("Siege entered with a protector")
}

pub(super) fn knight(e: &Engine<RegistryLookup>) -> Option<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.controller == PlayerId::new(0) && o.card.is_none())
        })
}

pub(super) fn assert_nonlethal(e: &Engine<RegistryLookup>, battle: ObjectId, damage: u16) {
    assert_eq!(defense(e, battle), 5 - damage);
    assert_eq!(
        e.state().object(battle).unwrap().damage,
        0,
        "a pure Battle gets no creature damage marks"
    );
    assert_eq!(e.state().object(battle).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().players[0].life, 20);
    assert_eq!(e.state().players[1].life, 20);
    let enemy = on_battlefield(e, PlayerId::new(1), fangren_hunter()).unwrap();
    assert_eq!(e.state().object(enemy).unwrap().damage, 0);
    assert_eq!(e.state().object(knight(e).unwrap()).unwrap().damage, 0);
}

pub(super) fn await_defeat_cast(e: &mut Engine<RegistryLookup>, battle: ObjectId) {
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::PlayFromExile,
                ..
            }
        )
    });
    assert!(
        matches!(e.pending(), Pending::ChooseCards {
        player, options, min: 0, max: 1,
        prompt: crate::choice::ChoicePrompt::PlayFromExile, ..
    } if *player == PlayerId::new(0) && options.as_slice() == [battle]),
        "{:?}",
        e.pending()
    );
    assert_eq!(
        e.state().object(battle).unwrap().zone,
        Zone::Exile,
        "defeat trigger exiles before offering the back cast"
    );
}

pub(super) fn accept_back(e: &mut Engine<RegistryLookup>, battle: ObjectId) {
    e.apply(
        PlayerId::new(0),
        PlayerAction::ChooseObjects {
            objects: vec![battle],
        },
    )
    .unwrap();
    pass_until(e, |e| {
        e.state().zones.stack_is_empty() && matches!(e.pending(), Pending::Priority { .. })
    });
    let object = e.state().object(battle).unwrap();
    assert_eq!(object.zone, Zone::Battlefield);
    assert_eq!(object.face_index, 1);
    assert!(
        object
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    assert!(!object.characteristics().types.contains(TypeSet::BATTLE));
    assert_eq!(
        pt(e, knight(e).unwrap()),
        (3, 3),
        "the actually cast back face supplies its anthem"
    );
}
