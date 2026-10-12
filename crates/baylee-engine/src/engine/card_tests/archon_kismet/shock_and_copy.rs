#[allow(clippy::wildcard_imports)]
use super::*;

static GATE_UNTAP: &[baylee_cards_dsl::AbilityDef] = &[baylee_cards_dsl::AbilityDef::Replacement(
    baylee_cards_dsl::ReplacementRule::EnterUntapped(&baylee_cards_dsl::Filter::And(&[
        baylee_cards_dsl::Filter::HasSubtype(subtypes::land::GATE),
        baylee_cards_dsl::Filter::ControlledByYou,
    ])),
)];
fn install_untap_fixture(e: &mut Engine<RegistryLookup>, source: ObjectId) {
    let acquisition = e.state.next_timestamp();
    e.state
        .object_mut(source)
        .unwrap()
        .take_abilities(crate::object::AbilityList {
            acquisition,
            abilities: crate::copiable_abilities::AbilityDefs::from_static(GATE_UNTAP),
            printed: None,
            token: None,
        });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
}
fn replacement(e: &mut Engine<RegistryLookup>, source: ObjectId) {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt: crate::choice::ChoicePrompt::EntryReplacement,
        ..
    } = e.pending()
    else {
        panic!("entry replacement order")
    };
    assert_eq!(
        *player, P1,
        "affected permanent's entering controller chooses"
    );
    assert_eq!((*min, *max), (1, 1));
    assert!(options.contains(&source));
    e.apply(
        P1,
        PlayerAction::ChooseObjects {
            objects: vec![source],
        },
    )
    .unwrap();
}

#[test]
fn paying_real_shockland_life_does_not_override_existing_kismet() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::KISMET])
        .hand(1, &[index::HALLOWED_FOUNTAIN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    next_main(&mut e, P1);
    let land = in_hand(&e, P1, index::HALLOWED_FOUNTAIN).unwrap();
    let life = e.state.players[1].life;
    e.apply(P1, PlayerAction::PlayLand { card: land }).unwrap();
    // If ordering is offered, choose the land's printed replacement first.
    if matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: crate::choice::ChoicePrompt::EntryReplacement,
            ..
        }
    ) {
        e.apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
    }
    assert!(
        matches!(e.pending(),Pending::YesNo{player,prompt:crate::choice::YesNoPrompt::PayLifeOrEnterTapped{amount:2},..} if *player==P1)
    );
    e.apply(P1, PlayerAction::YesNo(true)).unwrap();
    if matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: crate::choice::ChoicePrompt::EntryReplacement,
            ..
        }
    ) {
        let kismet = obj(&e, P0, index::KISMET);
        e.apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![kismet],
            },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 2);
    assert!(
        is_tapped(&e, land),
        "paying only removes the shockland's own tap instruction; Kismet still applies"
    );
}

#[test]
fn actually_cast_copy_artifact_copies_worn_powerstones_printed_tapped_entry() {
    let mut e = prepared(&[index::COPY_ARTIFACT], &[index::WORN_POWERSTONE], &[]);
    let target = obj(&e, P0, index::WORN_POWERSTONE);
    let copier = in_hand(&e, P0, index::COPY_ARTIFACT).unwrap();
    cast(&mut e, P0, index::COPY_ARTIFACT);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![target], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        is_tapped(&e, copier),
        "the copied enters-tapped ability is copiable even though the spell began as an enchantment"
    );
}

#[test]
fn copied_guildgate_printed_tap_is_ordered_with_external_untap_fixture() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(1, &[index::SOL_RING, index::AZORIUS_GUILDGATE])
        .hand(1, &[index::COPY_ARTIFACT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let fixture = obj(&e, P1, index::SOL_RING);
    install_untap_fixture(&mut e, fixture);
    let target = obj(&e, P1, index::AZORIUS_GUILDGATE);
    // Copy Artifact only copies artifacts; this isolated type exception makes
    // the actual Guildgate a legal copy choice, without inventing Gond support.
    let base = e.state.object_mut(target).unwrap().base_mut();
    base.types = base.types.union(baylee_core::types::TypeSet::ARTIFACT);
    e.state.invalidate_projections();
    e.refresh_offer();
    next_main(&mut e, P1);
    let copier = in_hand(&e, P1, index::COPY_ARTIFACT).unwrap();
    cast(&mut e, P1, index::COPY_ARTIFACT);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![target], vec![]);
    replacement(&mut e, copier);
    if matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: crate::choice::ChoicePrompt::EntryReplacement,
            ..
        }
    ) {
        replacement(&mut e, fixture);
    }
    assert_eq!(e.state.object(copier).unwrap().zone, Zone::Battlefield);
    assert!(
        !is_tapped(&e, copier),
        "copied printed tap first, external Gate untap last"
    );
}

#[test]
fn replacement_choice_checkpoint_resumes_the_same_remaining_order() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::KISMET])
        .battlefield(1, &[index::SOL_RING])
        .hand(1, &[index::AZORIUS_GUILDGATE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let fixture = obj(&e, P1, index::SOL_RING);
    install_untap_fixture(&mut e, fixture);
    next_main(&mut e, P1);
    let land = in_hand(&e, P1, index::AZORIUS_GUILDGATE).unwrap();
    let kismet = obj(&e, P0, index::KISMET);
    e.apply(P1, PlayerAction::PlayLand { card: land }).unwrap();
    replacement(&mut e, land);
    assert!(matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: crate::choice::ChoicePrompt::EntryReplacement,
            ..
        }
    ));
    let hash = e.snapshot_hash();
    let checkpoint = crate::engine::checkpoint::Checkpoint::capture(&e);
    replacement(&mut e, kismet);
    if matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: crate::choice::ChoicePrompt::EntryReplacement,
            ..
        }
    ) {
        replacement(&mut e, fixture);
    }
    let finished = e.snapshot_hash();
    assert!(!is_tapped(&e, land));
    checkpoint.restore(&mut e);
    assert_eq!(e.snapshot_hash(), hash);
    replacement(&mut e, kismet);
    if matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: crate::choice::ChoicePrompt::EntryReplacement,
            ..
        }
    ) {
        replacement(&mut e, fixture);
    }
    assert!(!is_tapped(&e, land));
    assert_eq!(
        e.snapshot_hash(),
        finished,
        "applied-source history, incoming incarnation, question and remaining order survive checkpoint restore"
    );
}

#[test]
fn kismet_arriving_in_same_entry_event_does_not_tap_its_companion() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::KISMET])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let kismet = in_hand(&e, P0, index::KISMET).unwrap();
    let bear = in_hand(&e, P1, index::GRIZZLY_BEARS).unwrap();
    // Explicit simultaneous-instruction event fixture. No registered completed
    // helper was verified to put these opposing permanents in together (Eureka
    // remains Partial). Freeze one begin_event cursor for both arrivals.
    e.state
        .entries
        .begin_event(e.state.journal.last_seq(), Some(P0));
    e.state
        .move_object(
            kismet,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    // Recompute rules between internal commits deliberately: the event boundary,
    // rather than absence of a sync, must exclude its newly arrived replacement.
    e.sync_static_effects();
    e.state
        .move_object(
            bear,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert!(
        !is_tapped(&e, bear),
        "replacement effects must exist before the simultaneous event; a co-arriving Kismet cannot affect its companion"
    );
}

// Append to engine/card_tests/archon_kismet/shock_and_copy.rs.
// Transparent fixture: the co-arriving enchantment has only the creature
// ability-removal static below. No unimplemented helper card is used.
static COARRIVING_CREATURE_ABILITY_LOSS: &[baylee_cards_dsl::AbilityDef] =
    &[baylee_cards_dsl::static_ability!(
        baylee_cards_dsl::Filter::CREATURE,
        baylee_cards_dsl::Modifier::LoseAllAbilities
    )];

#[test]
fn coarriving_ability_removal_does_not_disable_an_existing_archons_entry_replacement() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ARCHON_OF_EMERIA])
        .hand(0, &[index::KISMET])
        .hand(1, &[index::COMMAND_TOWER])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let archon = obj(&e, P0, index::ARCHON_OF_EMERIA);
    let fixture = in_hand(&e, P0, index::KISMET).unwrap();
    let land = in_hand(&e, P1, index::COMMAND_TOWER).unwrap();

    // As in kismet_arriving_in_same_entry_event_does_not_tap_its_companion,
    // one explicit event models the simultaneous instruction. Internal commits
    // and static synchronization must not make new continuous effects influence
    // replacement effects for another participant in that same event.
    e.state
        .entries
        .begin_event(e.state.journal.last_seq(), Some(P0));
    e.state
        .move_object(
            fixture,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    assert!(crate::entry::advance(&mut e.state).is_none());
    assert_eq!(
        e.state.object(fixture).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    let acquisition = e.state.next_timestamp();
    e.state
        .object_mut(fixture)
        .unwrap()
        .take_abilities(crate::object::AbilityList {
            acquisition,
            abilities: crate::copiable_abilities::AbilityDefs::from_static(
                COARRIVING_CREATURE_ABILITY_LOSS,
            ),
            printed: None,
            token: None,
        });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.state.refresh_characteristics();
    e.sync_static_effects();
    assert!(
        e.state
            .object(archon)
            .unwrap()
            .characteristics()
            .abilities_lost
            .is_some()
    );

    e.state
        .move_object(
            land,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    assert!(crate::entry::advance(&mut e.state).is_none());
    assert_eq!(
        e.state.object(land).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert!(
        is_tapped(&e, land),
        "the existing Archon applies before the co-arriving ability-loss static exists"
    );
    e.sync_static_effects();
    e.state.refresh_characteristics();
    e.sync_static_effects();
    assert!(
        e.state
            .object(archon)
            .unwrap()
            .characteristics()
            .abilities_lost
            .is_some(),
        "after the event the fixture genuinely removes the Archon's abilities"
    );
}
