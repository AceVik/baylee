//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

use crate::event::Cause;
use crate::zone::ZonePosition;
use baylee_cards_dsl::KeywordSet;

fn float(e: &mut Engine<RegistryLookup>, color: ManaColor, amount: u32) {
    e.dev_state_mut(PlayerId::new(0)).unwrap().players[0]
        .mana_pool
        .add(color, amount);
    e.refresh_offer();
}
fn blink(e: &mut Engine<RegistryLookup>, object: ObjectId) {
    let s = e.dev_state_mut(PlayerId::new(0)).unwrap();
    s.move_object(
        object,
        ZoneLocation::Exile(PlayerId::new(0)),
        ZonePosition::Top,
        Cause::Effect,
    )
    .unwrap();
    s.object_mut(object).unwrap().kind = crate::object::ObjectKind::Permanent;
    s.move_object(
        object,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    )
    .unwrap();
}

#[test]
fn forum_only_adds_counters_to_a_multicolored_creature_and_blink_ends_the_bonus() {
    let p0 = PlayerId::new(0);
    for (card, multi) in [
        (index::MARHAULT_ELSDRAGON, true),
        (index::GRIZZLY_BEARS, false),
        (index::MIND_STONE, false),
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::GUILDMAGES_FORUM, index::DOUBLING_SEASON])
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        float(&mut e, ManaColor::Colorless, 1);
        activate(&mut e, p0, index::GUILDMAGES_FORUM, 1);
        e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
            .unwrap();
        if multi {
            float(&mut e, ManaColor::Red, 2);
            float(&mut e, ManaColor::Colorless, 3);
        } else {
            float(&mut e, ManaColor::Colorless, 1);
        }
        cast_with_floating(&mut e, p0, card);
        pass_until(&mut e, stack_is_empty);
        let object = on_battlefield(&e, p0, card).unwrap();
        assert_eq!(
            counters_on(&e, object, CounterKind::P1P1),
            if multi { 2 } else { 0 }
        );
        blink(&mut e, object);
        e.refresh_offer();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            counters_on(&e, object, CounterKind::P1P1),
            0,
            "later incarnation received no paid mana"
        );
    }
}

#[test]
fn palace_counts_this_commander_including_this_cast_and_ignores_a_partners_casts() {
    let p0 = PlayerId::new(0);
    for (previous, from_hand) in [(0, false), (3, false), (3, true), (0, true)] {
        let mut e = Duel::new(SEED, forest())
            .commander(
                0,
                &[
                    index::SHEOLDRED_THE_APOCALYPSE,
                    index::JIN_GITAXIAS_PROGRESS_TYRANT,
                ],
            )
            .battlefield(0, &[index::OPAL_PALACE, index::DOUBLING_SEASON])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let commander = e.state().commanders[0][0].object;
        {
            let s = e.dev_state_mut(p0).unwrap();
            s.commanders[0][0].casts = previous;
            s.commanders[0][1].casts = 17;
            if from_hand {
                s.move_object(
                    commander,
                    ZoneLocation::Hand(p0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            }
        }
        float(&mut e, ManaColor::Colorless, 1);
        activate(&mut e, p0, index::OPAL_PALACE, 1);
        if matches!(e.pending(), Pending::ChooseColor { .. }) {
            e.apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
                .unwrap();
        }
        float(&mut e, ManaColor::Black, 1);
        float(
            &mut e,
            ManaColor::Colorless,
            2 + if from_hand { 0 } else { 2 * previous },
        );
        e.apply(p0, PlayerAction::CastSpell { card: commander })
            .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(commander).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            u32::from(counters_on(&e, commander, CounterKind::P1P1)),
            2 * (previous + u32::from(!from_hand))
        );
        blink(&mut e, commander);
        e.refresh_offer();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(counters_on(&e, commander, CounterKind::P1P1), 0);
    }
}

#[test]
fn study_hall_scries_four_above_the_spell_after_its_mana_source_has_left() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .commander(0, &[index::SHEOLDRED_THE_APOCALYPSE])
        .battlefield(0, &[index::STUDY_HALL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let commander = e.state().commanders[0][0].object;
    e.dev_state_mut(p0).unwrap().commanders[0][0].casts = 3;
    let hall = on_battlefield(&e, p0, index::STUDY_HALL).unwrap();
    float(&mut e, ManaColor::Colorless, 1);
    activate(&mut e, p0, index::STUDY_HALL, 1);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            hall,
            ZoneLocation::Hand(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    float(&mut e, ManaColor::Black, 1);
    float(&mut e, ManaColor::Colorless, 8);
    e.apply(p0, PlayerAction::CastSpell { card: commander })
        .unwrap();
    let stack = e.state().zones.list(ZoneLocation::Stack);
    assert_eq!(stack.len(), 2);
    assert_eq!(stack[0], commander);
    assert_eq!(
        e.state().object(stack[1]).unwrap().ability.unwrap().source,
        hall
    );
    pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange {
        player,
        cards,
        prompt,
        ..
    } = e.pending().clone()
    else {
        panic!("scry question")
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(cards.len(), 4);
    assert_eq!(e.state().object(commander).unwrap().zone, Zone::Stack);
    e.apply(
        p0,
        PlayerAction::Arrange {
            piles: vec![cards, vec![]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(commander).unwrap().zone, Zone::Battlefield);
}

#[test]
fn bandit_haste_follows_the_spell_indefinitely_but_not_a_blink() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::HALL_OF_THE_BANDIT_LORD])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let hall = on_battlefield(&e, p0, index::HALL_OF_THE_BANDIT_LORD).unwrap();
    let life = e.state().players[0].life;
    activate(&mut e, p0, index::HALL_OF_THE_BANDIT_LORD, 0);
    assert_eq!(e.state().players[0].life, life - 3);
    float(&mut e, ManaColor::Green, 1);
    cast_with_floating(&mut e, p0, index::GRIZZLY_BEARS);
    pass_until(&mut e, stack_is_empty);
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    assert!(keywords(&e, bear).contains(KeywordSet::HASTE));
    bury(&mut e, &[hall]);
    let turn = e.state().turn.number;
    pass_until(&mut e, |e| {
        e.state().turn.number > turn && e.state().turn.phase == Phase::FirstMain
    });
    assert!(keywords(&e, bear).contains(KeywordSet::HASTE));
    blink(&mut e, bear);
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert!(!keywords(&e, bear).contains(KeywordSet::HASTE));
}

#[test]
fn crossroads_pays_life_and_restricts_colored_mana_to_devoid_spells() {
    use crate::casting::{SpendFor, pay_mana_for, spendable_units};
    use baylee_core::mana::ManaCost;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::CORRUPTED_CROSSROADS])
        .hand(
            0,
            &[
                index::DROWNER_OF_TRUTH,
                index::ORNITHOPTER,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let life = e.state().players[0].life;
    activate(&mut e, p0, index::CORRUPTED_CROSSROADS, 1);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(e.state().players[0].life, life - 1);
    let drowner = in_hand(&e, p0, index::DROWNER_OF_TRUTH).unwrap();
    assert_eq!(spendable_units(e.state(), p0, SpendFor::Spell(drowner)), 1);
    for card in [index::ORNITHOPTER, index::GRIZZLY_BEARS] {
        let id = in_hand(&e, p0, card).unwrap();
        assert_eq!(spendable_units(e.state(), p0, SpendFor::Spell(id)), 0);
        assert!(
            pay_mana_for(
                e.dev_state_mut(p0).unwrap(),
                p0,
                SpendFor::Spell(id),
                &ManaCost::parse("{1}")
            )
            .is_none()
        );
    }
    assert!(
        pay_mana_for(
            e.dev_state_mut(p0).unwrap(),
            p0,
            SpendFor::Ability(drowner),
            &ManaCost::parse("{1}")
        )
        .is_none()
    );
    assert_eq!(e.state().players[0].mana_pool.total(), 1);
}

#[test]
fn cavern_uncounterability_does_not_follow_a_bounced_card_into_its_next_cast() {
    use baylee_core::generated::subtypes::creature;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::CAVERN_OF_SOULS])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    e.apply(p0, PlayerAction::ChooseSubtype(creature::BEAR))
        .unwrap();
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, index::CAVERN_OF_SOULS, 1);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    float(&mut e, ManaColor::Colorless, 1);
    cast_with_floating(&mut e, p0, index::GRIZZLY_BEARS);
    let bear = on_stack(&e, index::GRIZZLY_BEARS).unwrap();
    assert!(
        e.state()
            .object(bear)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Uncounterable)
    );
    pass_until(&mut e, stack_is_empty);
    assert!(
        !e.state()
            .object(bear)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Uncounterable)
    );
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            bear,
            ZoneLocation::Hand(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    float(&mut e, ManaColor::Green, 2);
    cast_with_floating(&mut e, p0, index::GRIZZLY_BEARS);
    assert!(
        !e.state()
            .object(bear)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Uncounterable)
    );
}

#[test]
fn transferred_path_mana_still_uses_its_original_controller_for_the_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .commander(0, &[index::VENSER_SHAPER_SAVANT])
        .battlefield(0, &[index::PATH_OF_ANCESTRY])
        .hand(1, &[index::SNAPCASTER_MAGE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, index::PATH_OF_ANCESTRY, 0);
    {
        let s = e.dev_state_mut(p0).unwrap();
        let (a, b) = s.players.split_at_mut(1);
        assert!(a[0].mana_pool.transfer_to(&mut b[0].mana_pool));
        b[0].mana_pool.add(ManaColor::Colorless, 1);
    }
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut e, p1, index::SNAPCASTER_MAGE);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange { player, cards, .. } = e.pending() else {
        panic!("scry")
    };
    assert_eq!(*player, p0);
    assert_eq!(cards.len(), 1);
    assert!(
        e.state()
            .zones
            .list(ZoneLocation::Library(p0))
            .contains(&cards[0])
    );
}

#[test]
fn bandit_haste_uses_layer_six_timestamp_order_instead_of_becoming_a_printed_keyword() {
    use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use baylee_cards_dsl::{Duration, Modifier};
    let p0 = PlayerId::new(0);
    for removal_is_newer in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::HALL_OF_THE_BANDIT_LORD])
            .hand(0, &[index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        activate(&mut e, p0, index::HALL_OF_THE_BANDIT_LORD, 0);
        float(&mut e, ManaColor::Green, 1);
        cast_with_floating(&mut e, p0, index::GRIZZLY_BEARS);
        let bear = on_stack(&e, index::GRIZZLY_BEARS).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        let timestamp = if removal_is_newer {
            state.next_timestamp()
        } else {
            0
        };
        let modifier = Modifier::LoseAllAbilities;
        let filter = EffectFilter::object(state, bear);
        state.effects.register(ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: p0,
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: Duration::Indefinitely,
            filter,
            modifier,
        });
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            keywords(&e, bear).contains(KeywordSet::HASTE),
            !removal_is_newer
        );
    }
}

/// "When you spend this mana to cast your commander, scry X": spending it on
/// anything else scries nothing, even in a game where a commander has been
/// cast from the command zone three times.
#[test]
fn study_hall_mana_spent_on_a_noncommander_spell_scries_nothing() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .commander(0, &[index::SHEOLDRED_THE_APOCALYPSE])
        .battlefield(0, &[index::STUDY_HALL])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().commanders[0][0].casts = 3;
    float(&mut e, ManaColor::Colorless, 1);
    activate(&mut e, p0, index::STUDY_HALL, 1);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Green),
        1
    );

    cast_with_floating(&mut e, p0, llanowar_elves());
    assert_eq!(
        e.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "only the Elves: casting a noncommander spell with the mana puts no scry trigger up"
    );
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
            || (on_battlefield(e, p0, llanowar_elves()).is_some() && stack_is_empty(e))
    });
    assert!(
        !matches!(e.pending(), Pending::Arrange { .. }),
        "no scry, whatever X would have been: {:?}",
        e.pending()
    );
    assert!(on_battlefield(&e, p0, llanowar_elves()).is_some());
}
