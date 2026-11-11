//! `cards/planeswalkers/mv_3/vivien_champion_of_the_wilds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::partials_w1b_kit as kit;

fn vivien() -> CardIndex {
    card_index("146eb8c8-93d5-4dd5-af7a-46d5acd4a89d")
}

#[track_caller]
fn activate_loyalty(engine: &mut Engine<RegistryLookup>, vivien: ObjectId, nth: usize) {
    let offered = kit::offered(engine, vivien);
    let index = *offered
        .get(nth)
        .unwrap_or_else(|| panic!("loyalty ability {nth} is offered: {offered:?}"));
    kit::activate_object(engine, kit::P0, vivien, index);
}

#[track_caller]
fn float(engine: &mut Engine<RegistryLookup>, color: ManaColor, amount: u32) {
    engine.dev_state_mut(kit::P0).unwrap().players[0]
        .mana_pool
        .add(color, amount);
    engine.refresh_offer();
}

/// Creature spells have flash on either turn; the static ability says nothing
/// about noncreature spells, even when their mana is available.
#[test]
fn vivien_gives_creature_spells_flash_but_not_noncreature_spells() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vivien()])
        .hand(
            0,
            &[llanowar_elves(), baylee_core::generated::index::DOOMSKAR],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    float(&mut engine, ManaColor::Green, 1);
    float(&mut engine, ManaColor::White, 5);

    let elf = in_hand(&engine, p0, llanowar_elves()).expect("Elf in hand");
    let sorcery =
        in_hand(&engine, p0, baylee_core::generated::index::DOOMSKAR).expect("Doomskar in hand");
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected player 0 priority: {:?}", engine.pending())
    };
    assert!(legal.castable.contains(&elf), "creature spell has flash");
    assert!(
        !legal.castable.contains(&sorcery),
        "a sorcery remains noncastable even with all its mana available"
    );
    assert!(
        baylee_engine::casting::can_cast(engine.state(), &RegistryLookup, p0, sorcery).is_err()
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: elf })
        .expect("the creature can be cast on the opponent's turn");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(kit::zone_of(&engine, elf), Zone::Battlefield);
}

/// +1 targets up to one creature, not every creature, and its two keywords
/// survive the opponent's turn but end as Vivien's controller's next turn starts.
#[test]
fn vivien_plus_one_gives_only_the_target_reach_and_vigilance_until_the_next_turn() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vivien(), gray_ogre()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let source = on_battlefield(&engine, p0, vivien()).expect("Vivien");
    let target = on_battlefield(&engine, p0, gray_ogre()).expect("target Ogre");
    let other = on_battlefield(&engine, p1, llanowar_elves()).expect("other creature");
    let turn = engine.state().turn.number;

    activate_loyalty(&mut engine, source, 0);
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "up to one target still permits a target: {:?}",
        engine.pending()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("one creature is a legal up-to-one target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(kit::loyalty(&engine, source), 5);
    let granted = KeywordSet::REACH.union(KeywordSet::VIGILANCE);
    assert!(keywords(&engine, target).contains(granted));
    assert!(!keywords(&engine, other).contains(KeywordSet::REACH));
    assert!(!keywords(&engine, other).contains(KeywordSet::VIGILANCE));

    reach_their_main_phase(&mut engine, p1);
    assert!(
        keywords(&engine, target).contains(granted),
        "the grant lasts through the opponent's turn"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.number > turn + 1
            && e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(!keywords(&engine, target).contains(KeywordSet::REACH));
    assert!(
        !keywords(&engine, target).contains(KeywordSet::VIGILANCE),
        "until your next turn has expired"
    );
}

/// "Up to one" is a legal zero-target activation and still pays +1 loyalty.
#[test]
fn vivien_plus_one_may_choose_no_target() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vivien(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let source = on_battlefield(&engine, p0, vivien()).expect("Vivien");

    activate_loyalty(&mut engine, source, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .expect("zero targets is legal for up to one");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(kit::loyalty(&engine, source), 5);
}

/// Resolves −2 with an exact one-card private choice and a deliberately
/// reversed bottom order. Returns the exiled object and all inspected objects.
#[track_caller]
fn resolve_minus_two(
    top_first: &[CardIndex],
    selected: usize,
) -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>) {
    let p0 = kit::P0;
    assert!(top_first.len() >= 3 && selected < 3);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vivien(), forest(), forest(), forest()])
        .hand(0, top_first)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let source = on_battlefield(&engine, p0, vivien()).expect("Vivien");
    let stacked = kit::stack_library(&mut engine, p0, top_first);

    activate_loyalty(&mut engine, source, 1);
    assert_eq!(kit::loyalty(&engine, source), 2, "−2 is paid on activation");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1), "exile exactly one, not up to one");
    assert_eq!(options, stacked[..3], "only the top three are options");
    assert_eq!(engine.inspected_cards(), &stacked[..3]);
    let exiled = stacked[selected];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![exiled],
            },
        )
        .unwrap();

    let Pending::Arrange { cards, piles, .. } = engine.pending().clone() else {
        panic!(
            "the other two cards need a bottom order: {:?}",
            engine.pending()
        )
    };
    assert_eq!(piles.len(), 1);
    let mut order: Vec<_> = stacked[..3]
        .iter()
        .copied()
        .filter(|card| *card != exiled)
        .collect();
    assert_eq!(cards, order);
    order.reverse();
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![order.clone()],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let library = engine.state().zones.list(ZoneLocation::Library(p0));
    let bottom_first: Vec<_> = order.iter().rev().copied().collect();
    assert_eq!(
        &library[..2],
        &bottom_first,
        "the chosen top-to-bottom order survives bottom insertion"
    );
    let object = engine.state().object(exiled).expect("the exiled card");
    assert_eq!(object.zone, Zone::Exile);
    assert!(object.status.contains(Status::FACE_DOWN));
    assert!(object.characteristics().types.is_empty());
    assert!(object.may_look_at_exile(p0));
    assert!(!object.may_look_at_exile(kit::P1));
    (engine, exiled, stacked)
}

/// The creature remains castable after later turns, is revealed to cast, and
/// pays its ordinary mana cost rather than entering for free.
#[test]
fn vivien_minus_two_exiles_a_private_creature_that_can_be_cast_later_for_mana() {
    let p0 = kit::P0;
    let (mut engine, exiled, _) =
        resolve_minus_two(&[forest(), gray_ogre(), lightning_bolt(), island()], 1);
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float(&mut engine, ManaColor::Red, 3);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority: {:?}", engine.pending())
    };
    assert!(legal.castable.contains(&exiled));

    engine
        .apply(p0, PlayerAction::CastSpell { card: exiled })
        .expect("the exact exiled creature incarnation remains castable");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    }
    let spell = engine.state().object(exiled).expect("the creature spell");
    assert_eq!(spell.zone, Zone::Stack);
    assert!(!spell.status.contains(Status::FACE_DOWN));
    assert!(spell.characteristics().types.contains(TypeSet::CREATURE));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}}{{R}} was paid"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(kit::zone_of(&engine, exiled), Zone::Battlefield);
}

/// The same permission never makes a noncreature exiled by −2 castable.
#[test]
fn vivien_minus_two_does_not_allow_casting_an_exiled_noncreature() {
    let p0 = kit::P0;
    let (mut engine, exiled, _) =
        resolve_minus_two(&[lightning_bolt(), gray_ogre(), forest(), island()], 0);
    float(&mut engine, ManaColor::Red, 1);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority: {:?}", engine.pending())
    };
    assert!(!legal.castable.contains(&exiled));
    assert!(baylee_engine::casting::can_cast(engine.state(), &RegistryLookup, p0, exiled).is_err());
    let before = engine.snapshot_hash();
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: exiled })
            .is_err()
    );
    assert_eq!(engine.snapshot_hash(), before);
    assert!(
        engine
            .state()
            .object(exiled)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
}

/// Announcing reveals the card, but cancelling before its mana is made puts
/// the same hidden incarnation back without losing its printable creature face.
#[test]
fn vivien_minus_two_cancelled_cast_conceals_the_same_creature_again() {
    let p0 = kit::P0;
    let (mut engine, exiled, _) =
        resolve_minus_two(&[forest(), gray_ogre(), lightning_bolt(), island()], 1);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority: {:?}", engine.pending())
    };
    assert!(legal.payable.contains(&exiled), "three Forests can pay");
    engine
        .apply(p0, PlayerAction::CastSpell { card: exiled })
        .expect("the creature may be announced before making its mana");
    let announced = engine.state().object(exiled).expect("announced exile");
    assert_eq!(announced.zone, Zone::Exile);
    assert!(!announced.status.contains(Status::FACE_DOWN));
    assert!(
        announced
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert_eq!(engine.cancellable_cast(p0), Some(exiled));

    engine.apply(p0, PlayerAction::CancelCast).unwrap();
    let concealed = engine.state().object(exiled).expect("cancelled exile");
    assert!(concealed.status.contains(Status::FACE_DOWN));
    assert!(concealed.characteristics().types.is_empty());
    assert!(
        concealed
            .original_base
            .as_ref()
            .is_some_and(|base| base.types.contains(TypeSet::CREATURE))
    );
    assert!(concealed.may_look_at_exile(p0));
    assert!(!concealed.may_look_at_exile(kit::P1));
}
