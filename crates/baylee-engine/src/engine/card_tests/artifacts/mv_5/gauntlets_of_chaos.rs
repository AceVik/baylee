//! `cards/artifacts/mv_5/gauntlets_of_chaos.rs`, played.
//!
//! Written from the Oracle text, the Scryfall rulings (2009-10-01) and the
//! Comprehensive Rules alone (CR 701.12 "Exchange", 608.2b, 602.2b, 303.4).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::aura_kit as kit;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

// oracle_id = "924ef69f-9977-439c-ace9-353b429e4be6"
fn gauntlets() -> CardIndex {
    card_index("924ef69f-9977-439c-ace9-353b429e4be6")
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const WHITE: (ManaColor, u32) = (ManaColor::White, 1);

/// A duel at seat 0's first main phase: the Gauntlets, five Forests to pay
/// for them and whatever else the scenario sets out.
#[track_caller]
fn table(
    my_board: &[CardIndex],
    their_board: &[CardIndex],
    hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut board = vec![gauntlets(); 1];
    board.extend([index::FOREST; 5]);
    board.extend_from_slice(my_board);
    let mut engine = Duel::new(SEED, index::FOREST)
        .battlefield(0, &board)
        .battlefield(1, their_board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, P0),
        "seat 0 reaches its own main"
    );
    engine
}

/// Taps exactly the five Forests, so that `{5}` is floating and nothing else
/// has been spent.
#[track_caller]
fn pay_five(engine: &mut Engine<RegistryLookup>) {
    let forests = all_on_battlefield(engine, P0, index::FOREST);
    let tapped = tap_mana_where(engine, P0, |id| forests.contains(&id));
    assert_eq!(tapped, 5, "five Forests, five mana");
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
}

#[track_caller]
fn ours(engine: &Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    on_battlefield(engine, P0, card).expect("seat 0 controls it")
}

#[track_caller]
fn theirs(engine: &Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    on_battlefield(engine, P1, card).expect("seat 1 controls it")
}

/// "{5}, Sacrifice this artifact: Exchange control of target artifact,
/// creature, or land you control and target permanent an opponent controls
/// that shares one of those types with it. If those permanents are exchanged
/// this way, destroy all Auras attached to them."
///
/// My Bears and their Hill Giant trade places (the very same objects: an
/// exchange moves nothing between zones, CR 701.12b), the Gauntlets are in the
/// graveyard and `{5}` is gone, and **every** Aura attached to either of the
/// two is destroyed -- mine on the Bears, mine on their Giant -- while an
/// Aura on a third creature, one that was never a target, stays attached.
#[test]
fn gauntlets_exchange_two_creatures_and_destroy_the_auras_on_both() {
    let mut engine = table(
        &[index::GRIZZLY_BEARS, index::GRAY_OGRE],
        &[index::HILL_GIANT, index::CRAW_WURM],
        &[index::HOLY_STRENGTH; 3],
    );
    let (bears, ogre) = (
        ours(&engine, index::GRIZZLY_BEARS),
        ours(&engine, index::GRAY_OGRE),
    );
    let (giant, wurm) = (
        theirs(&engine, index::HILL_GIANT),
        theirs(&engine, index::CRAW_WURM),
    );
    let on_bears = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, bears, &[WHITE]);
    let on_giant = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, giant, &[WHITE]);
    let on_ogre = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, ogre, &[WHITE]);
    assert_eq!(pt(&engine, bears), (3, 4), "Holy Strength is +1/+2");
    assert_eq!(pt(&engine, giant), (4, 5));
    assert_eq!(pt(&engine, ogre), (3, 4));

    pay_five(&mut engine);
    activate(&mut engine, P0, gauntlets(), 0);
    let first = aim_at(&mut engine, P0, bears);
    assert!(first.contains(&bears), "a creature I control: {first:?}");
    assert!(
        !first.contains(&giant),
        "but never one of theirs: {first:?}"
    );
    let second = aim_at(&mut engine, P0, giant);
    assert!(
        second.contains(&giant),
        "a creature they control: {second:?}"
    );
    assert!(
        !second.contains(&bears),
        "and not one of mine, for all that it is a creature: {second:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P0, gauntlets()).is_some()
            && on_battlefield(&engine, P0, gauntlets()).is_none(),
        "the Gauntlets were sacrificed"
    );
    assert_eq!(
        kit::controller_of(&engine, bears),
        P1,
        "my Bears are theirs"
    );
    assert_eq!(
        kit::controller_of(&engine, giant),
        P0,
        "their Giant is mine"
    );
    assert_eq!(
        kit::controller_of(&engine, wurm),
        P1,
        "the creature that was never a target did not move"
    );
    assert_eq!(
        kit::controller_of(&engine, ogre),
        P0,
        "nor did my other one"
    );
    assert!(
        !kit::on_the_table(&engine, on_bears) && !kit::on_the_table(&engine, on_giant),
        "the Auras on the two exchanged permanents are destroyed"
    );
    assert_eq!(
        kit::count_in(&engine, ZoneLocation::Graveyard(P0), index::HOLY_STRENGTH),
        2,
        "both went to their owner's graveyard"
    );
    assert!(
        kit::on_the_table(&engine, on_ogre) && kit::attached_to(&engine, on_ogre) == Some(ogre),
        "an Aura on a permanent that was not exchanged is still where it was"
    );
    assert_eq!(pt(&engine, bears), (2, 2), "the Bears are plain again");
    assert_eq!(pt(&engine, giant), (3, 3), "and so is the Giant");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{5}} was spent"
    );
}

/// "Exchange control of target artifact, creature, or land you control and
/// target permanent an opponent controls that shares one of those types with
/// it." The second menu is read for each kind of first target, every time with
/// the right answers and the wrong ones side by side:
///
/// - a creature: their creature, not their land or their artifact;
/// - a land: their land, not their creature;
/// - an artifact: their artifact, not their land;
/// - an artifact creature: their artifact *and* their creature (it shares
///   one type with each), still not their land.
///
/// Nothing of mine is ever on the second menu.
#[test]
fn gauntlets_second_target_must_share_a_type_with_the_first() {
    // (first target, must be on the second menu, must not be)
    let cases: [(CardIndex, &[CardIndex], &[CardIndex]); 4] = [
        (
            index::GRIZZLY_BEARS,
            &[index::HILL_GIANT],
            &[index::MOUNTAIN, index::SOL_RING],
        ),
        (
            index::PLAINS,
            &[index::MOUNTAIN],
            &[index::HILL_GIANT, index::SOL_RING],
        ),
        (
            index::SOL_RING,
            &[index::SOL_RING],
            &[index::HILL_GIANT, index::MOUNTAIN],
        ),
        (
            index::ORNITHOPTER,
            &[index::SOL_RING, index::HILL_GIANT],
            &[index::MOUNTAIN],
        ),
    ];
    for (first, wanted, unwanted) in cases {
        let mut engine = table(
            &[
                index::GRIZZLY_BEARS,
                index::PLAINS,
                index::SOL_RING,
                index::ORNITHOPTER,
            ],
            &[index::HILL_GIANT, index::MOUNTAIN, index::SOL_RING],
            &[],
        );
        let mine_first = on_battlefield(&engine, P0, first).expect("it is on my side");
        pay_five(&mut engine);
        activate(&mut engine, P0, gauntlets(), 0);
        let first_menu = aim_at(&mut engine, P0, mine_first);
        assert!(
            first_menu.contains(&mine_first),
            "the first menu holds my {first:?}: {first_menu:?}"
        );
        for id in &first_menu {
            assert_eq!(
                kit::controller_of(&engine, *id),
                P0,
                "the first menu only ever holds permanents I control"
            );
        }
        let Pending::ChooseTargets { options: menu, .. } = engine.pending().clone() else {
            panic!("the second target is asked: {:?}", engine.pending());
        };
        for card in wanted {
            let id = on_battlefield(&engine, P1, *card).expect("it is on their side");
            assert!(
                menu.contains(&id),
                "first target {first:?}: their {card:?} shares a type with it and is offered: {menu:?}"
            );
        }
        for card in unwanted {
            let id = on_battlefield(&engine, P1, *card).expect("it is on their side");
            assert!(
                !menu.contains(&id),
                "first target {first:?}: their {card:?} shares no type with it and is not offered: {menu:?}"
            );
        }
        for id in &menu {
            assert_eq!(
                kit::controller_of(&engine, *id),
                P1,
                "the second menu only ever holds permanents an opponent controls"
            );
        }
    }
}

/// Lands and artifacts trade too, and an Aura on a land is destroyed with the
/// exchange: my Plains (with Wild Growth on it) for their Mountain, and my Sol
/// Ring for their Ornithopter -- an artifact for an artifact *creature*,
/// which shares the artifact type with it.
#[test]
fn gauntlets_exchange_lands_and_artifacts_as_well() {
    let mut lands = table(&[index::PLAINS], &[index::MOUNTAIN], &[index::WILD_GROWTH]);
    let (plains, mountain) = (ours(&lands, index::PLAINS), theirs(&lands, index::MOUNTAIN));
    let growth = kit::cast_aura(
        &mut lands,
        P0,
        index::WILD_GROWTH,
        plains,
        &[(ManaColor::Green, 1)],
    );
    pay_five(&mut lands);
    activate(&mut lands, P0, gauntlets(), 0);
    aim_at(&mut lands, P0, plains);
    aim_at(&mut lands, P0, mountain);
    pass_until(&mut lands, stack_is_empty);
    assert_eq!(
        kit::controller_of(&lands, plains),
        P1,
        "my Plains went over"
    );
    assert_eq!(
        kit::controller_of(&lands, mountain),
        P0,
        "their Mountain came back"
    );
    assert!(
        !kit::on_the_table(&lands, growth),
        "the Aura on the exchanged land is destroyed"
    );

    let mut rocks = table(&[index::SOL_RING], &[index::ORNITHOPTER], &[]);
    let (ring, thopter) = (
        ours(&rocks, index::SOL_RING),
        theirs(&rocks, index::ORNITHOPTER),
    );
    pay_five(&mut rocks);
    activate(&mut rocks, P0, gauntlets(), 0);
    aim_at(&mut rocks, P0, ring);
    aim_at(&mut rocks, P0, thopter);
    pass_until(&mut rocks, stack_is_empty);
    assert_eq!(
        kit::controller_of(&rocks, ring),
        P1,
        "my Sol Ring is theirs"
    );
    assert_eq!(
        kit::controller_of(&rocks, thopter),
        P0,
        "their Ornithopter is mine"
    );
}

/// Ruling 2009-10-01: "If one of the targets is illegal by the time the
/// ability resolves ..., the exchange doesn't happen. The target that's still
/// legal will remain under its controller's control. Since the exchange
/// doesn't happen, no Auras are destroyed." (CR 608.2b, 701.12a.)
///
/// Either target is made illegal by a Lightning Bolt cast in response, once
/// the Gauntlets have already been sacrificed for their cost: the survivor is
/// where it was, and the Aura on it is still attached.
#[test]
fn gauntlets_do_nothing_when_a_target_is_gone_by_resolution() {
    for second_dies in [true, false] {
        // The survivor wears a Holy Strength (cast by me, so a destroyed
        // Aura would be in my graveyard); the other target is a 2/2 that a
        // Bolt kills.
        let (survivor_card, victim_card) = if second_dies {
            (index::HILL_GIANT, index::GRIZZLY_BEARS)
        } else {
            (index::GRIZZLY_BEARS, index::HILL_GIANT)
        };
        let (mine_cards, their_cards) = if second_dies {
            (vec![survivor_card, index::MOUNTAIN], vec![victim_card])
        } else {
            (vec![victim_card, index::MOUNTAIN], vec![survivor_card])
        };
        let mut engine = table(
            &mine_cards,
            &their_cards,
            &[index::HOLY_STRENGTH, index::LIGHTNING_BOLT],
        );
        let (survivor, victim) = if second_dies {
            (ours(&engine, survivor_card), theirs(&engine, victim_card))
        } else {
            (theirs(&engine, survivor_card), ours(&engine, victim_card))
        };
        // In the second case the Bears are mine and the survivor is theirs;
        // either way the Aura goes on the survivor.
        let wearing = kit::cast_aura(&mut engine, P0, index::HOLY_STRENGTH, survivor, &[WHITE]);
        let controller_before = kit::controller_of(&engine, survivor);

        pay_five(&mut engine);
        activate(&mut engine, P0, gauntlets(), 0);
        let (first_pick, second_pick) = if second_dies {
            (survivor, victim)
        } else {
            (victim, survivor)
        };
        aim_at(&mut engine, P0, first_pick);
        aim_at(&mut engine, P0, second_pick);
        assert!(
            in_graveyard(&engine, P0, gauntlets()).is_some(),
            "the sacrifice is a cost: the Gauntlets are already gone, ability still on the stack"
        );
        assert!(!stack_is_empty(&engine), "the ability waits on the stack");

        // In response: a Bolt for the victim.
        let mountain = ours(&engine, index::MOUNTAIN);
        assert!(
            !is_tapped(&engine, mountain),
            "the Mountain was kept back for the Bolt"
        );
        cast_from_hand(&mut engine, P0, index::LIGHTNING_BOLT);
        aim_at(&mut engine, P0, victim);
        pass_until(&mut engine, stack_is_empty);

        assert!(
            !kit::on_the_table(&engine, victim),
            "positive control: the Bolt really did kill the target (second_dies = {second_dies})"
        );
        assert_eq!(
            kit::controller_of(&engine, survivor),
            controller_before,
            "the target that is still legal stays under its controller's control \
             (second_dies = {second_dies})"
        );
        assert!(
            kit::on_the_table(&engine, wearing)
                && kit::attached_to(&engine, wearing) == Some(survivor),
            "no exchange, so no Aura is destroyed (second_dies = {second_dies})"
        );
    }
}

/// "{5}": the ability is on the offer with five mana and not with four. Four
/// Forests is the whole board of the second duel, so the refusal is for want
/// of the fifth mana and for no other reason; the first duel (five Forests,
/// otherwise identical) is the control that it is offered.
#[test]
fn gauntlets_cost_five_mana() {
    let mut five = table(&[index::GRIZZLY_BEARS], &[index::HILL_GIANT], &[]);
    pay_five(&mut five);
    assert!(
        lk::offered(&five, gauntlets(), P0, 0),
        "with {{5}} floating the ability is offered"
    );

    // Four Forests only.
    let mut engine = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                gauntlets(),
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));
    let forests = all_on_battlefield(&engine, P0, index::FOREST);
    assert_eq!(forests.len(), 4);
    assert_eq!(
        tap_mana_where(&mut engine, P0, |id| forests.contains(&id)),
        4
    );
    assert!(
        !lk::offered(&engine, gauntlets(), P0, 0),
        "with four mana it is not"
    );
}
