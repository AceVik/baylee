//! Connive/recruit use completed draws, defined discard characteristics and exact incarnations.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::{Cause, GameEvent};
use crate::zone::ZonePosition;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ready(hand: &[CardIndex], battlefield: &[CardIndex]) -> Engine<RegistryLookup> {
    prepared(
        Duel::new(SEED, index::FOREST)
            .hand(0, hand)
            .hand(1, &[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT])
            .battlefield(0, battlefield),
    )
}

fn prepared(duel: Duel) -> Engine<RegistryLookup> {
    let mut e = duel.start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for seat in [P0, P1] {
        let pool = &mut e.dev_state_mut(seat).unwrap().players[usize::from(seat.get())].mana_pool;
        for color in [
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Blue,
            ManaColor::Colorless,
        ] {
            pool.add(color, 20);
        }
    }
    e.refresh_offer();
    e
}

fn bolt(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    cast_with_floating(e, seat, index::LIGHTNING_BOLT);
    e.apply(
        seat,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P1],
        },
    )
    .unwrap();
}

fn discard(e: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    let id = in_hand(e, P0, card).unwrap();
    let Pending::ChooseCards {
        player,
        min,
        max,
        options,
        ..
    } = e.pending()
    else {
        panic!("{:?}", e.pending())
    };
    assert_eq!((*player, *min, *max), (P0, 1, 1));
    assert!(options.contains(&id));
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![id] })
        .unwrap();
    id
}

fn connives(e: &Engine<RegistryLookup>) -> usize {
    e.journal()
        .entries()
        .iter()
        .filter(|entry| matches!(entry.event, GameEvent::Connived { .. }))
        .count()
}

#[test]
fn looting_shredder_observes_each_players_second_spell_and_rewards_only_nonlands() {
    for caster in [P0, P1] {
        for discarded in [index::FOREST, index::GRIZZLY_BEARS] {
            let mut e = ready(
                &[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT, discarded],
                &[index::LEDGER_SHREDDER],
            );
            let shredder = on_battlefield(&e, P0, index::LEDGER_SHREDDER).unwrap();
            if caster == P1 {
                e.apply(P0, PlayerAction::PassPriority).unwrap();
            }
            bolt(&mut e, caster);
            assert_eq!(connives(&e), 0);
            // Keep priority, so both spells are genuinely cast in this turn.
            bolt(&mut e, caster);
            let library = library_size(&e, P0);
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseCards { .. })
            });
            assert_eq!(library_size(&e, P0), library - 1);
            discard(&mut e, discarded);
            pass_until(&mut e, stack_is_empty);
            let counter = i16::from(discarded == index::GRIZZLY_BEARS);
            assert_eq!(pt(&e, shredder), (1 + counter, 3 + counter));
            assert_eq!(connives(&e), 1);
        }
    }
}

#[test]
fn looting_recruit_creates_a_white_human_soldier_and_respects_token_doublers() {
    for doubling in [false, true] {
        let mut field = vec![index::LAKE_TOWN_LOOKOUT];
        if doubling {
            field.push(index::DOUBLING_SEASON);
        }
        let mut e = ready(&[index::LIGHTNING_BOLT, index::GRIZZLY_BEARS], &field);
        let lookout = on_battlefield(&e, P0, index::LAKE_TOWN_LOOKOUT).unwrap();
        cast_with_floating(&mut e, P0, index::LIGHTNING_BOLT);
        aim_at(&mut e, P0, lookout);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        discard(&mut e, index::GRIZZLY_BEARS);
        pass_until(&mut e, stack_is_empty);
        let tokens = tokens_of(&e, P0);
        assert_eq!(tokens.len(), if doubling { 2 } else { 1 });
        for token in tokens {
            let c = e.state().object(token).unwrap().characteristics();
            assert_eq!(pt(&e, token), (1, 1));
            assert_eq!(
                c.colors,
                baylee_core::color::ColorSet::of(baylee_core::color::Color::White)
            );
            for subtype in [
                baylee_core::generated::subtypes::creature::HUMAN,
                baylee_core::generated::subtypes::creature::SOLDIER,
            ] {
                assert!(c.subtypes.contains(subtype));
            }
        }
    }
}

#[test]
fn looting_hidden_discard_has_no_nonland_characteristics() {
    for recruit in [false, true] {
        let source = if recruit {
            index::LAKE_TOWN_LOOKOUT
        } else {
            index::LEDGER_SHREDDER
        };
        let mut e = ready(
            &[
                index::LIGHTNING_BOLT,
                index::LIGHTNING_BOLT,
                index::GRIZZLY_BEARS,
            ],
            &[source, index::LIBRARY_OF_LENG],
        );
        let permanent = on_battlefield(&e, P0, source).unwrap();
        if recruit {
            cast_with_floating(&mut e, P0, index::LIGHTNING_BOLT);
            aim_at(&mut e, P0, permanent);
        } else {
            bolt(&mut e, P0);
            bolt(&mut e, P0);
        }
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let bear = discard(&mut e, index::GRIZZLY_BEARS);
        assert!(matches!(e.pending(), Pending::Arrange { .. }));
        e.apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![vec![], vec![bear]],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().zones.list(ZoneLocation::Library(P0)).last(),
            Some(&bear)
        );
        assert!(e.journal().entries().iter().any(|entry| matches!(entry.event, GameEvent::Discarded { object, known: false, .. } if object == bear)));
        if recruit {
            assert!(tokens_of(&e, P0).is_empty());
        } else {
            assert_eq!(pt(&e, permanent), (1, 3));
            assert_eq!(connives(&e), 1);
        }
    }
}

#[test]
fn looting_departed_shredder_still_connives_without_putting_counters_on_a_new_incarnation() {
    let mut e = ready(
        &[
            index::LIGHTNING_BOLT,
            index::LIGHTNING_BOLT,
            index::GRIZZLY_BEARS,
        ],
        &[index::LEDGER_SHREDDER],
    );
    let shredder = on_battlefield(&e, P0, index::LEDGER_SHREDDER).unwrap();
    bolt(&mut e, P0);
    bolt(&mut e, P0);
    let old_version = e.state().object(shredder).unwrap().version;
    let state = e.dev_state_mut(P0).unwrap();
    state
        .move_object(
            shredder,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            shredder,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    discard(&mut e, index::GRIZZLY_BEARS);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, shredder), (1, 3));
    assert!(e.journal().entries().iter().any(|entry| matches!(entry.event, GameEvent::Connived { object, .. } if object.object == shredder && object.version == old_version)));
}

#[test]
fn looting_vault_counts_zero_and_more_than_255_deaths_without_partial_discards() {
    for deaths in [0_u32, 1, 257] {
        let mut hand = vec![index::FOREST; 300];
        hand.push(index::GRIZZLY_BEARS);
        let mut e = ready(
            &hand,
            &[
                index::SPYMASTER_S_VAULT,
                index::SWAMP,
                index::LEDGER_SHREDDER,
                index::DOUBLING_SEASON,
            ],
        );
        // Supply a library large enough for the boundary; no fabricated loot events.
        let cards = e.state().zones.list(ZoneLocation::Hand(P0)).clone();
        for card in cards.iter().take(300) {
            e.dev_state_mut(P0)
                .unwrap()
                .move_object(
                    *card,
                    ZoneLocation::Library(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        e.dev_state_mut(P0).unwrap().per_turn.creatures_died = deaths;
        e.refresh_offer();
        let shredder = on_battlefield(&e, P0, index::LEDGER_SHREDDER).unwrap();
        activate(&mut e, P0, index::SPYMASTER_S_VAULT, 1);
        aim_at(&mut e, P0, shredder);
        if deaths == 0 {
            pass_until(&mut e, stack_is_empty);
            assert_eq!(connives(&e), 0);
            continue;
        }
        let library = library_size(&e, P0);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        assert_eq!(
            library_size(&e, P0),
            library - usize::try_from(deaths).unwrap()
        );
        let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
        let mut total = 0;
        while let Pending::ChooseCards {
            options, min, max, ..
        } = e.pending().clone()
        {
            assert_eq!(min, max);
            assert!(
                !e.journal()
                    .entries()
                    .iter()
                    .any(|entry| matches!(entry.event, GameEvent::Discarded { .. })),
                "no discard until every card has been chosen"
            );
            let mut chosen = options;
            chosen.sort_by_key(|id| (*id != bear, *id));
            chosen.truncate(usize::from(min));
            total += chosen.len();
            e.apply(P0, PlayerAction::ChooseObjects { objects: chosen })
                .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(total, usize::try_from(deaths).unwrap());
        assert_eq!(pt(&e, shredder), (3, 5), "one known nonland, doubled");
        assert_eq!(connives(&e), 1);
    }
}

#[test]
fn looting_hideout_connives_only_a_villain_you_control_at_sorcery_speed() {
    let mut e = ready(
        &[index::GRIZZLY_BEARS],
        &[
            index::VILLAINOUS_HIDEOUT,
            index::MASKWOOD_NEXUS,
            index::LEDGER_SHREDDER,
        ],
    );
    let shredder = on_battlefield(&e, P0, index::LEDGER_SHREDDER).unwrap();
    activate(&mut e, P0, index::VILLAINOUS_HIDEOUT, 2);
    assert_eq!(aim_at(&mut e, P0, shredder), vec![shredder]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    discard(&mut e, index::GRIZZLY_BEARS);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, shredder), (2, 4));
}

mod discarded;
mod random;

#[test]
fn looting_finishes_replacement_draw_questions_before_the_discard_choice() {
    let mut e = ready(
        &[index::GRIZZLY_BEARS, index::FOREST],
        &[
            index::ISLAND_SANCTUARY,
            index::SPYMASTER_S_VAULT,
            index::SWAMP,
            index::LEDGER_SHREDDER,
        ],
    );
    for _ in 0..200 {
        if e.state().turn.active == P0
            && e.state().turn.step == crate::turn::Step::Draw
            && matches!(e.pending(), Pending::Priority { player: P0, .. })
        {
            break;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => e.apply(player, PlayerAction::YesNo(true)).unwrap(),
            Pending::ChooseAttackers { player, .. } => e
                .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                .unwrap(),
            Pending::ChooseBlockers { player, .. } => e
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .unwrap(),
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(e.state().turn.step, crate::turn::Step::Draw);
    let state = e.dev_state_mut(P0).unwrap();
    state.per_turn.creatures_died = 2;
    state.players[0].mana_pool.add(ManaColor::Black, 1);
    e.refresh_offer();
    let shredder = on_battlefield(&e, P0, index::LEDGER_SHREDDER).unwrap();
    let before = library_size(&e, P0);
    activate(&mut e, P0, index::SPYMASTER_S_VAULT, 1);
    aim_at(&mut e, P0, shredder);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert!(
        matches!(e.pending(), Pending::YesNo { .. }),
        "second draw is offered before any discard"
    );
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(library_size(&e, P0), before - 1);
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let forest = in_hand(&e, P0, index::FOREST).unwrap();
    assert!(matches!(
        e.pending(),
        Pending::ChooseCards { min: 2, max: 2, .. }
    ));
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![bear, forest],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, shredder), (2, 4));
    assert_eq!(connives(&e), 1);
}

#[test]
fn looting_second_spell_uses_its_event_ordinal_when_several_casts_are_collected_together() {
    let mut e = ready(&[], &[index::LEDGER_SHREDDER]);
    let state = e.dev_state_mut(P0).unwrap();
    let from = state.journal.last_seq();
    for (number, caster) in [(1, P0), (1, P1), (2, P0), (3, P0), (2, P1)] {
        let name = state.names.intern("Test spell");
        let object = state.create_bare(
            caster,
            crate::object::ObjectKind::Spell,
            name,
            ZoneLocation::Stack,
        );
        state.journal.record(GameEvent::SpellCast {
            instant_number: 0,
            object,
            player: caster,
            number,
        });
        state.per_turn.spells_cast[usize::from(caster.get())] = number;
    }
    let triggers = crate::trigger::collect(state, &RegistryLookup, from);
    assert_eq!(
        triggers.len(),
        2,
        "each player's second cast triggers once, independent of later casts"
    );
}
