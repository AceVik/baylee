//! Linked abilities acquired together retain their own exile family.
#[allow(clippy::wildcard_imports)]
use super::*;

// No registered creature has this linked activated pair. The fixture supplies
// only two immutable DSL definitions to creature donors, then uses
// actual Cauldron grants/actions for every exile and mana production below.
static LINKED_ACTIVATED_PAIR: &[baylee_cards_dsl::AbilityDef] = &[
    baylee_cards_dsl::activated!(
        baylee_cards_dsl::Cost::TAP,
        &[baylee_cards_dsl::Effect::ExileTargetsWithSource],
        target = Some(baylee_cards_dsl::TargetSpec::CardInGraveyard(
            &baylee_cards_dsl::Filter::Any,
            baylee_cards_dsl::PlayerRel::EachPlayer
        ))
    ),
    baylee_cards_dsl::mana_ability!(&[baylee_cards_dsl::Effect::mana_exiled_colors()]),
];

fn install_linked_pair(e: &mut Engine<RegistryLookup>, donor: ObjectId) {
    let acquisition = e.state.next_timestamp();
    e.state
        .object_mut(donor)
        .unwrap()
        .take_abilities(crate::object::AbilityList {
            acquisition,
            abilities: crate::copiable_abilities::AbilityDefs::from_static(LINKED_ACTIVATED_PAIR),
            printed: None,
            token: None,
        });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
}

fn linked_read(
    e: &mut Engine<RegistryLookup>,
    recipient: ObjectId,
    id: u32,
    color: Option<ManaColor>,
) {
    e.state.set_tapped(recipient, false);
    reset_pool(e);
    use_grant(e, recipient, id);
    assert!(
        stack_is_empty(e),
        "the paired read is an immediate mana ability"
    );
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        u64::from(color.is_some())
    );
    if let Some(color) = color {
        assert_eq!(e.state.players[0].mana_pool.available(color), 1);
    }
    assert!(is_tapped(e, recipient));
}

fn linked_exile(e: &mut Engine<RegistryLookup>, recipient: ObjectId, id: u32, card: ObjectId) {
    e.state.set_tapped(recipient, false);
    reset_pool(e);
    use_grant(e, recipient, id);
    target(e, vec![card], vec![]);
    pass_until(e, stack_is_empty);
    assert_eq!(e.state.object(card).unwrap().zone, Zone::Exile);
}

#[test]
#[allow(clippy::too_many_lines)] // Two linked pairs across donor/recipient acquisitions, played end to end.
fn batch46_agatha_linked_activated_pairs_share_only_their_own_granted_acquisition() {
    let mut e = prepared(
        &[
            index::FIEND_ARTISAN,
            index::COUNTERSPELL,
            index::LIGHTNING_BOLT,
            index::LIGHTNING_BOLT,
            index::GIANT_GROWTH,
        ],
        &[
            index::AGATHA_S_SOUL_CAULDRON,
            index::GRIZZLY_BEARS,
            index::SHIVAN_DRAGON,
        ],
        &[],
    );
    let first = on_battlefield(&e, P0, index::SHIVAN_DRAGON).unwrap();
    let second = hand_to_graveyard(&mut e, P0, index::FIEND_ARTISAN);
    let red = hand_to_graveyard(&mut e, P0, index::LIGHTNING_BOLT);
    let new_red = hand_to_graveyard(&mut e, P0, index::LIGHTNING_BOLT);
    let green = hand_to_graveyard(&mut e, P0, index::GIANT_GROWTH);
    let bearer = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let blue = hand_to_graveyard(&mut e, P0, index::COUNTERSPELL);
    // First exercise the fixture donor's own pair through actual actions.
    // Cards it exiled before becoming a donor are not inherited by a bearer.
    install_linked_pair(&mut e, first);
    linked_exile(&mut e, first, 0, blue);
    linked_read(&mut e, first, 1, Some(ManaColor::Blue));
    e.state
        .move_object(
            first,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    exile(&mut e, first, Some(bearer));
    install_linked_pair(&mut e, first);
    let a = grants(&e, bearer);
    assert_eq!(a.len(), 2);
    exile(&mut e, second, Some(bearer));
    install_linked_pair(&mut e, second);
    let b: Vec<_> = grants(&e, bearer)
        .into_iter()
        .filter(|id| !a.contains(id))
        .collect();
    assert_eq!(b.len(), 2);
    linked_read(&mut e, bearer, a[1], None);
    linked_exile(&mut e, bearer, a[0], red);
    linked_read(&mut e, bearer, b[1], None);
    linked_read(&mut e, bearer, a[1], Some(ManaColor::Red));
    linked_exile(&mut e, bearer, b[0], green);
    linked_read(&mut e, bearer, b[1], Some(ManaColor::Green));
    linked_read(&mut e, bearer, a[1], Some(ManaColor::Red));

    // The same acquired pair on a returned recipient cannot read what the
    // recipient's previous incarnation exiled, even while the cards remain.
    e.state
        .move_object(
            bearer,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            bearer,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .object_mut(bearer)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 1);
    e.state.invalidate_projections();
    e.sync_static_effects();
    // Reentry gives the bearer summoning sickness. Advance through actual
    // priority/combat actions until it has begun a new turn under our control.
    let blink_turn = e.state.turn.number;
    pass_until(&mut e, |e| {
        e.state.turn.number > blink_turn
            && e.state.turn.active == P0
            && matches!(e.state.turn.phase, crate::turn::Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    linked_read(&mut e, bearer, a[1], None);
    linked_read(&mut e, bearer, b[1], None);
    linked_exile(&mut e, bearer, a[0], new_red);
    linked_read(&mut e, bearer, a[1], Some(ManaColor::Red));

    // Re-exiling a donor creates a new grant acquisition, independently of
    // the recipient's incarnation and the donor's identical DSL definitions.
    e.state
        .move_object(
            first,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    exile(&mut e, first, Some(bearer));
    install_linked_pair(&mut e, first);
    let new_a: Vec<_> = grants(&e, bearer)
        .into_iter()
        .filter(|id| !b.contains(id))
        .collect();
    assert_eq!(new_a.len(), 2);
    assert!(!new_a.iter().any(|id| a.contains(id)));
    linked_read(&mut e, bearer, new_a[1], None);
    assert_eq!(e.state.object(red).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.object(new_red).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.object(green).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.object(blue).unwrap().zone, Zone::Exile);
}

static FIXED_LINKED_READER: &[baylee_cards_dsl::AbilityDef] = &[baylee_cards_dsl::static_ability!(
    baylee_cards_dsl::Filter::YOUR_CREATURE,
    baylee_cards_dsl::Modifier::GrantActivatedDefinition(&LINKED_ACTIVATED_PAIR[1])
)];

#[test]
fn batch46_agatha_single_linked_clause_cannot_read_recipients_original_family() {
    let mut e = prepared(
        &[index::LIGHTNING_BOLT],
        &[index::AGATHA_S_SOUL_CAULDRON, index::GRIZZLY_BEARS],
        &[],
    );
    let bearer = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let red = hand_to_graveyard(&mut e, P0, index::LIGHTNING_BOLT);
    install_linked_pair(&mut e, bearer);
    linked_exile(&mut e, bearer, 0, red);
    linked_read(&mut e, bearer, 1, Some(ManaColor::Red));

    // This explicit DSL fixture grants only the reader, without its linked
    // exile clause. CR 607.5a gives it no access to the original pair's choice.
    let source = cauldron(&e);
    let acquisition = e.state.next_timestamp();
    e.state
        .object_mut(source)
        .unwrap()
        .take_abilities(crate::object::AbilityList {
            acquisition,
            abilities: crate::copiable_abilities::AbilityDefs::from_static(FIXED_LINKED_READER),
            printed: None,
            token: None,
        });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    let ids = grants(&e, bearer);
    assert_eq!(ids.len(), 1);
    linked_read(&mut e, bearer, ids[0], None);
    linked_read(&mut e, bearer, 1, Some(ManaColor::Red));
    assert_eq!(e.state.object(red).unwrap().zone, Zone::Exile);
}
