//! Entry replacement choices publish no partial move and survive checkpoints.
//! Rules checked against Wizards' 2026-09-25 CR, 614.12a and 614.16:
//! <https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt>
use super::synthetic::{self, SyntheticLookup};
use super::*;
use crate::choice::YesNoPrompt;
use crate::event::{Cause, GameEvent};
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::*;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::DeckEntry;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);
const RETURNER: u32 = 995_720;
const LONG_SAGA: u32 = 995_721;

fn card(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).expect("fixture card exists")
}

fn deck_entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn fixture(hand: &[CardIndex]) -> Engine<SyntheticLookup> {
    static RETURN: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[Effect::AllGraveyardCreaturesToBattlefield]
    )];
    // An inert late chapter keeps the transaction test independent of printed
    // chapter choices and the final-chapter sacrifice while advancing turns.
    static CHAPTERS: &[AbilityDef] = &[AbilityDef::SagaChapter {
        chapter: 12,
        effects: &[],
        targets: None,
    }];
    let mut face = synthetic::land_face("Entry transaction Saga");
    face.types = TypeSet::ENCHANTMENT;
    // An empty inherited land cost is unpayable; this spell prints {0}.
    face.mana_cost = baylee_core::mana!("{0}");
    face.subtypes = &[baylee_core::generated::subtypes::enchantment::SAGA];
    face.castable_from_hand = true;
    let saga = Box::leak(Box::new(CardDef {
        index: CardIndex::new(LONG_SAGA),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([face])),
        color_identity: ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities: CHAPTERS,
    }));
    let mut preset = synthetic::preset(
        995_722,
        &[
            RETURNER,
            card("Brad Boimler, Eager Ensign").get(),
            card("Doubling Season").get(),
        ],
    );
    preset.seats[0].starting_hand = Some(hand.iter().copied().map(deck_entry).collect());
    let mut e = Engine::new(
        &preset,
        SyntheticLookup::new(vec![
            synthetic::land(RETURNER, "Mass entry probe", RETURN),
            saga,
        ]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut e);
    drive_until(&mut e, |e| {
        e.state.turn.active == ME && e.state.turn.phase == Phase::FirstMain
    });
    enable_brad(&mut e);
    e
}

#[track_caller]
fn drive_until(e: &mut Engine<SyntheticLookup>, done: impl Fn(&Engine<SyntheticLookup>) -> bool) {
    for _ in 0..200 {
        if done(e) {
            return;
        }
        let pending = e.pending().clone();
        assert!(
            synthetic::walk_past(e, &pending),
            "unexpected choice: {pending:?}"
        );
    }
    panic!(
        "transaction boundary not reached: pending={:?}, active={:?}, phase={:?}, step={:?}, stack={:?}, effects={:?}",
        e.pending(),
        e.state.turn.active,
        e.state.turn.phase,
        e.state.turn.step,
        e.state.zones.list(ZoneLocation::Stack),
        e.state
            .effects
            .iter()
            .map(|effect| &effect.modifier)
            .collect::<Vec<_>>(),
    );
}

fn find(e: &Engine<SyntheticLookup>, zone: ZoneLocation, index: CardIndex) -> ObjectId {
    e.state
        .zones
        .list(zone)
        .iter()
        .copied()
        .find(|id| {
            e.state
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == index)
        })
        .expect("fixture object exists")
}

fn enable_brad(e: &mut Engine<SyntheticLookup>) {
    let brad = find(
        e,
        ZoneLocation::Battlefield,
        card("Brad Boimler, Eager Ensign"),
    );
    // The tap is a fixture event. The real card's triggered ability must
    // resolve before any entry begins; its canonical card tests cover the tap cost.
    assert!(e.state.set_tapped(brad, true));
    e.state.journal.record(GameEvent::ObjectTapped {
        object: brad,
        cause: Cause::DevCommand,
    });
    e.refresh_offer();
    drive_until(e, |e| {
        e.state.effects.iter().any(|effect| {
            matches!(
                effect.modifier,
                Modifier::AdditionalCounterPlacement { n: 1 }
            )
        }) && e.state.zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { .. })
    });
}

fn order_question(e: &Engine<SyntheticLookup>) -> ObjectId {
    let Pending::YesNo {
        player: ME,
        prompt:
            YesNoPrompt::ApplyAdditionalCountersNext {
                permanent,
                additional: 1,
            },
        ..
    } = e.pending()
    else {
        panic!("entry ordering question: {:?}", e.pending());
    };
    *permanent
}

fn incoming_events(e: &Engine<SyntheticLookup>, id: ObjectId, since: u64) -> usize {
    e.journal().entries().iter().filter(|entry| entry.seq > since && matches!(
        entry.event, GameEvent::ZoneChanged { object, to: Zone::Battlefield, .. } if object == id
    )).count()
}

#[test]
fn entry_counter_order_waits_before_zone_change_and_checkpoint_resume_is_exact() {
    for (add_first, expected) in [(true, 4), (false, 3)] {
        let mut e = fixture(&[card("Spike Drone")]);
        e.state.players[0].mana_pool.add(ManaColor::Green, 1);
        e.refresh_offer();
        let incoming = find(&e, ZoneLocation::Hand(ME), card("Spike Drone"));
        e.apply(ME, PlayerAction::CastSpell { card: incoming })
            .unwrap();
        let version = e.state.object(incoming).unwrap().version;
        let since = e.journal().last_seq();
        drive_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        assert_eq!(order_question(&e), incoming);
        assert_eq!(e.state.object(incoming).unwrap().zone, Zone::Stack);
        assert_eq!(e.state.object(incoming).unwrap().version, version);
        assert_eq!(
            e.state
                .object(incoming)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            0
        );
        assert_eq!(incoming_events(&e, incoming, since), 0);
        let cloned = e.state.clone();
        assert_eq!(cloned.snapshot_hash(), e.state.snapshot_hash());
        let checkpoint = super::checkpoint::Checkpoint::capture(&e);
        e.apply(ME, PlayerAction::YesNo(add_first)).unwrap();
        drive_until(&mut e, |e| {
            e.state.zones.stack_is_empty() && matches!(e.pending(), Pending::Priority { .. })
        });
        assert_eq!(e.state.object(incoming).unwrap().zone, Zone::Battlefield);
        assert_eq!(e.state.object(incoming).unwrap().version, version + 1);
        assert_eq!(
            e.state
                .object(incoming)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            expected
        );
        assert_eq!(incoming_events(&e, incoming, since), 1);
        let final_fingerprint = e.fingerprint();
        let final_journal = format!("{:?}", e.journal().entries());
        checkpoint.restore(&mut e);
        assert_eq!(order_question(&e), incoming);
        e.apply(ME, PlayerAction::YesNo(add_first)).unwrap();
        drive_until(&mut e, |e| {
            e.state.zones.stack_is_empty() && matches!(e.pending(), Pending::Priority { .. })
        });
        assert_eq!(e.fingerprint(), final_fingerprint);
        assert_eq!(format!("{:?}", e.journal().entries()), final_journal);
    }
}

#[test]
fn simultaneous_entry_counter_choices_keep_every_incoming_object() {
    let drone = card("Spike Drone");
    let mut e = fixture(&[drone, drone]);
    let incoming: Vec<_> = e
        .state
        .zones
        .list(ZoneLocation::Hand(ME))
        .iter()
        .copied()
        .filter(|id| {
            e.state
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == drone)
        })
        .collect();
    assert_eq!(incoming.len(), 2);
    for id in &incoming {
        e.state
            .move_object(
                *id,
                ZoneLocation::Graveyard(ME),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();
    let source = find(&e, ZoneLocation::Battlefield, CardIndex::new(RETURNER));
    e.apply(
        ME,
        PlayerAction::ActivateAbility {
            source,
            ability_index: 0,
        },
    )
    .unwrap();
    let since = e.journal().last_seq();
    drive_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    for id in &incoming {
        assert_eq!(e.state.object(*id).unwrap().zone, Zone::Graveyard);
        assert_eq!(incoming_events(&e, *id, since), 0);
    }
    let first = order_question(&e);
    e.apply(ME, PlayerAction::YesNo(true)).unwrap();
    let second = order_question(&e);
    for id in &incoming {
        assert_eq!(e.state.object(*id).unwrap().zone, Zone::Graveyard);
        assert_eq!(
            incoming_events(&e, *id, since),
            0,
            "no coarrival commits before every entry choice"
        );
    }
    assert_ne!(first, second);
    assert!(incoming.contains(&first) && incoming.contains(&second));
    e.apply(ME, PlayerAction::YesNo(false)).unwrap();
    drive_until(&mut e, |e| {
        e.state.zones.stack_is_empty() && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        e.state
            .object(first)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4
    );
    assert_eq!(
        e.state
            .object(second)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3
    );
    for id in incoming {
        assert_eq!(e.state.object(id).unwrap().zone, Zone::Battlefield);
        assert_eq!(incoming_events(&e, id, since), 1);
    }
}

#[test]
fn saga_turn_based_lore_replacement_enables_effect_only_doubling() {
    let mut e = fixture(&[CardIndex::new(LONG_SAGA)]);
    let saga = find(&e, ZoneLocation::Hand(ME), CardIndex::new(LONG_SAGA));
    e.apply(ME, PlayerAction::CastSpell { card: saga }).unwrap();
    drive_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert_eq!(order_question(&e), saga);
    e.apply(ME, PlayerAction::YesNo(true)).unwrap();
    drive_until(&mut e, |e| {
        e.state.zones.stack_is_empty() && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        e.state
            .object(saga)
            .unwrap()
            .counters
            .get(CounterKind::Lore),
        4
    );
    drive_until(&mut e, |e| {
        e.state.turn.active == THEM && e.state.turn.phase == Phase::FirstMain
    });
    drive_until(&mut e, |e| {
        e.state.turn.active == ME
            && e.state.turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    enable_brad(&mut e);
    // CR 614.16: Brad replaces the turn-based +1 with +2. That replacement
    // is an effect, enabling Season to replace it with +4 (not +2 or +3).
    drive_until(&mut e, |e| e.state.turn.phase == Phase::FirstMain);
    drive_until(&mut e, |e| {
        e.state.zones.stack_is_empty() && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        e.state
            .object(saga)
            .unwrap()
            .counters
            .get(CounterKind::Lore),
        8
    );
}

#[test]
fn own_counter_maximum_caps_the_initial_entry_event_in_either_replacement_order() {
    // Rasputin's Oracle says it enters with seven dream counters and cannot
    // have more than seven. CR 614.17d applies that own-only prohibition
    // prospectively; an excess placement followed by an SBA removal is wrong.
    for add_first in [true, false] {
        let rasputin = card("Rasputin Dreamweaver");
        let mut e = fixture(&[rasputin]);
        e.state.players[0].mana_pool.add(ManaColor::White, 5);
        e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
        e.refresh_offer();
        let incoming = find(&e, ZoneLocation::Hand(ME), rasputin);
        e.apply(ME, PlayerAction::CastSpell { card: incoming })
            .unwrap();
        let since = e.journal().last_seq();
        drive_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        assert_eq!(order_question(&e), incoming);
        assert_eq!(e.state.object(incoming).unwrap().zone, Zone::Stack);
        assert_eq!(incoming_events(&e, incoming, since), 0);
        e.apply(ME, PlayerAction::YesNo(add_first)).unwrap();
        drive_until(&mut e, |e| {
            e.state.zones.stack_is_empty() && matches!(e.pending(), Pending::Priority { .. })
        });
        assert_eq!(e.state.object(incoming).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            e.state
                .object(incoming)
                .unwrap()
                .counters
                .get(counters::DREAM),
            7
        );
        let changes: Vec<_> = e
            .journal()
            .entries()
            .iter()
            .filter_map(|entry| {
                if entry.seq <= since {
                    return None;
                }
                match entry.event {
                    GameEvent::CounterChanged {
                        object,
                        kind: counters::DREAM,
                        old,
                        new,
                        ..
                    } if object == incoming => Some((old, new)),
                    _ => None,
                }
            })
            .collect();
        assert_eq!(
            changes,
            [(0, 7)],
            "publish exactly the capped placement, never 15/16 followed by removal"
        );
        assert_eq!(incoming_events(&e, incoming, since), 1);
    }
}
