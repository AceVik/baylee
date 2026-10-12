//! Transparent replacement-only fixture: Gond Gate remains Partial and is not
//! used as a supposedly completed helper. Real Kismet and Guildgate entry are tested.
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
fn kismet_and_external_untap_and_printed_tapped_are_ordered_by_the_entering_controller() {
    for untap_last in [false, true] {
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
        let incoming = in_hand(&e, P1, index::AZORIUS_GUILDGATE).unwrap();
        let kismet = obj(&e, P0, index::KISMET);
        e.apply(P1, PlayerAction::PlayLand { card: incoming })
            .unwrap();
        if untap_last {
            replacement(&mut e, incoming);
            replacement(&mut e, kismet);
        } else {
            replacement(&mut e, fixture);
            if matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: crate::choice::ChoicePrompt::EntryReplacement,
                    ..
                }
            ) {
                replacement(&mut e, incoming);
            }
        }
        // Sole remaining applicable replacement may auto-apply. If asked, choose it.
        if matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::EntryReplacement,
                ..
            }
        ) {
            replacement(&mut e, if untap_last { fixture } else { kismet });
        }
        assert_eq!(e.state.object(incoming).unwrap().zone, Zone::Battlefield);
        assert_eq!(is_tapped(&e, incoming), !untap_last);
    }
}
