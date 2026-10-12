//! Transparent two-mode spell fixture isolates the selected targeting domain.
//! These fixtures exercise engine actions; they do not implement the rules under test.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::synthetic::{self, SyntheticLookup};
use baylee_cards_dsl::{
    AbilityDef, CardDef, Effect, Filter, ModeCount, SpellMode, TargetReq, TargetSpec,
};
use baylee_core::generated::{index, subtypes};
const P0: PlayerId = PlayerId::new(0);
const MODAL: u32 = 98_720;
static WALL: Filter = Filter::HasSubtype(subtypes::creature::WALL);
static EFFECTS: &[Effect] = &[Effect::gain_life(1)];
static MODES: &[SpellMode] = &[
    baylee_cards_dsl::mode!(
        EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&WALL)))
    ),
    baylee_cards_dsl::mode!(
        EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),
];
static MODAL_ABILITIES: &[AbilityDef] = &[AbilityDef::ModalSpell {
    modes: MODES,
    choose: ModeCount::ONE,
}];
fn fixture(
    index_value: u32,
    original: CardIndex,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    let original = baylee_cards::by_index(original).unwrap();
    let face = baylee_cards_dsl::FaceDef {
        mana_cost: baylee_cards_dsl::mana!("{0}"),
        ..original.faces[0]
    };
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index_value),
        faces: Box::leak(Box::new([face])),
        abilities,
        ..*original
    }))
}
fn walk(e: &mut Engine<SyntheticLookup>, ready: impl Fn(&Engine<SyntheticLookup>) -> bool) {
    for _ in 0..300 {
        if ready(e) {
            return;
        }
        let pending = e.pending().clone();
        assert!(synthetic::walk_past(e, &pending), "{pending:?}");
    }
    panic!("not reached: {:?}", e.pending());
}
fn priority(e: &mut Engine<SyntheticLookup>) {
    walk(
        e,
        |e| matches!(e.pending(),Pending::Priority{player,..}if *player==P0),
    );
}
fn empty(e: &Engine<SyntheticLookup>) -> bool {
    e.state.zones.list(ZoneLocation::Stack).is_empty()
}
fn setup(def: &'static CardDef, hand: &[u32], battlefield: &[u32]) -> Engine<SyntheticLookup> {
    let mut preset = synthetic::preset(5277, battlefield);
    preset.seats[0].starting_hand = Some(
        hand.iter()
            .map(|c| baylee_core::preset::DeckEntry {
                card: CardIndex::new(*c),
                print: baylee_core::ids::PrintRef::new(0),
                rule_print: None,
                choices: baylee_core::card_metadata::DeckChoices::default(),
            })
            .collect(),
    );
    let mut e = Engine::new(&preset, SyntheticLookup::new(vec![def])).unwrap();
    synthetic::keep_mulligans(&mut e);
    walk(&mut e, |e| {
        e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player,..}if *player==P0)
    });
    for c in ManaColor::ALL {
        e.state.players[0].mana_pool.add(c, 30);
    }
    e.refresh_offer();
    e
}
fn object(e: &Engine<SyntheticLookup>, c: u32) -> ObjectId {
    synthetic::permanents(e, c)[0]
}
fn cast(e: &mut Engine<SyntheticLookup>, c: u32) {
    priority(e);
    let card = e
        .state
        .zones
        .list(ZoneLocation::Hand(P0))
        .iter()
        .copied()
        .find(|id| {
            e.state
                .object(*id)
                .unwrap()
                .card
                .is_some_and(|card| card.index.get() == c)
        })
        .unwrap();
    let pending = e.pending().clone();
    e.apply(P0, PlayerAction::CastSpell { card })
        .unwrap_or_else(|error| panic!("{c}: {error:?}, pending={pending:?}"));
}
fn target(e: &mut Engine<SyntheticLookup>, id: ObjectId) {
    let p = e.pending().asked().unwrap();
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![id],
            players: vec![],
        },
    )
    .unwrap();
}
#[test]
fn wall_permission_uses_selected_modal_domain_not_other_modes_or_current_candidates() {
    for mode in [0, 1] {
        let def = fixture(MODAL, index::LIGHTNING_BOLT, MODAL_ABILITIES);
        let mut e = setup(
            def,
            &[MODAL],
            &[index::WALL_OF_SHADOWS.get(), index::WALL_OF_STONE.get()],
        );
        let shadows = object(&e, index::WALL_OF_SHADOWS.get());
        let stone = object(&e, index::WALL_OF_STONE.get());
        cast(&mut e, MODAL);
        e.apply(P0, PlayerAction::ChooseMode(mode)).unwrap();
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            panic!("{:?}", e.pending())
        };
        assert_eq!(
            options.contains(&shadows),
            mode == 1,
            "only selected mode controls permission; all battlefield creatures are Walls"
        );
        target(&mut e, if mode == 0 { stone } else { shadows });
        walk(&mut e, empty);
    }
}

const TWO_SLOT: u32 = 98_722;
const GRANTOR: u32 = 98_723;

#[test]
fn wall_permission_uses_all_effective_target_slots_including_player_alternatives() {
    // The current engine's second slot is objects-only. Put the player
    // alternative in the FIRST slot, rather than inventing player slot 2.
    for kind in 0..3 {
        let (first, second) = match kind {
            0 => (
                TargetSpec::Object(&WALL),
                TargetSpec::Object(&Filter::ARTIFACT),
            ),
            1 => (TargetSpec::PlayerOrObject(&WALL), TargetSpec::Object(&WALL)),
            _ => (TargetSpec::Object(&WALL), TargetSpec::Object(&WALL)),
        };
        let abilities = Box::leak(Box::new([baylee_cards_dsl::spell!(
            EFFECTS,
            targets = Some(TargetReq::one(first)),
            second_targets = Some(TargetReq::one(second))
        )]));
        let def = fixture(TWO_SLOT, index::LIGHTNING_BOLT, abilities);
        let mut e = setup(
            def,
            &[TWO_SLOT],
            &[
                index::WALL_OF_SHADOWS.get(),
                index::WALL_OF_STONE.get(),
                index::SOL_RING.get(),
            ],
        );
        let shadow = object(&e, index::WALL_OF_SHADOWS.get());
        let stone = object(&e, index::WALL_OF_STONE.get());
        let ring = object(&e, index::SOL_RING.get());
        cast(&mut e, TWO_SLOT);
        let Pending::ChooseTargets {
            options,
            player_options,
            ..
        } = e.pending()
        else {
            panic!("first target slot: {:?}", e.pending());
        };
        assert_eq!(
            options.contains(&shadow),
            kind != 2,
            "a whole spell allowing artifacts or players is not Wall-only"
        );
        if kind == 1 {
            assert!(player_options.contains(&P0));
        }
        target(&mut e, if kind == 2 { stone } else { shadow });
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            panic!("second target slot: {:?}", e.pending());
        };
        if kind == 2 {
            assert!(!options.contains(&shadow));
        }
        target(&mut e, if kind == 0 { ring } else { stone });
        walk(&mut e, empty);
        assert_eq!(
            e.state.object(shadow).unwrap().zone,
            crate::zone::Zone::Battlefield
        );
    }
}

fn stacked_granted_wall_ability() -> Engine<SyntheticLookup> {
    let donor = baylee_cards::by_index(index::DWARVEN_DEMOLITION_TEAM).unwrap();
    let borrowed = Box::leak(Box::new(donor.abilities[0]));
    // Transparent fixture: a creature grants the REAL Demolition Team's full
    // activated definition to your creatures. Physical recipient is Sorcerer,
    // whose own printed ability has a broad target domain. Nothing here
    // implements Wall's restriction or supplies a fake frozen stack object.
    let grant_abilities = Box::leak(Box::new([baylee_cards_dsl::static_ability!(
        Filter::YOUR_CREATURE,
        baylee_cards_dsl::Modifier::GrantActivatedDefinition(borrowed)
    )]));
    let def = fixture(GRANTOR, index::GRIZZLY_BEARS, grant_abilities);
    let mut e = setup(
        def,
        &[
            index::UNSUMMON.get(),
            index::UNSUMMON.get(),
            index::VIRTUE_OF_KNOWLEDGE.get(),
        ],
        &[
            GRANTOR,
            index::PRODIGAL_SORCERER.get(),
            index::WALL_OF_SHADOWS.get(),
            index::WALL_OF_STONE.get(),
            index::WALL_OF_STONE.get(),
        ],
    );
    let recipient = object(&e, index::PRODIGAL_SORCERER.get());
    let shadow = object(&e, index::WALL_OF_SHADOWS.get());
    let stone = object(&e, index::WALL_OF_STONE.get());
    let grant = crate::effects::granted_activated(&e.state, recipient)
        .next()
        .unwrap();
    assert_eq!(grant.definition, *borrowed);
    priority(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: recipient,
            ability_index: grant.id,
        },
    )
    .unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("granted target");
    };
    assert!(!options.contains(&shadow));
    assert!(options.contains(&stone));
    target(&mut e, stone);
    let original = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    assert_eq!(
        e.state.object(original).unwrap().target_req,
        Some(TargetReq::one(TargetSpec::Object(&WALL))),
        "real granted announcement freezes its targeting requirement"
    );

    e
}

#[test]
fn granted_wall_only_definition_stays_frozen_after_recipient_and_grantor_leave_and_is_copied() {
    let mut e = stacked_granted_wall_ability();
    let grantor = object(&e, GRANTOR);
    let recipient = object(&e, index::PRODIGAL_SORCERER.get());
    let shadow = object(&e, index::WALL_OF_SHADOWS.get());
    let stone = object(&e, index::WALL_OF_STONE.get());
    let original = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();

    // Transparent recipient-ability-change fixture gives the already-selected
    // target the REAL Shadows static rules. Resolution must now recheck the
    // frozen Wall-only definition rather than Sorcerer's unrelated broad one.
    let wall_def = baylee_cards::by_index(index::WALL_OF_SHADOWS).unwrap();
    let acquisition = e.state.next_timestamp();
    e.state
        .object_mut(stone)
        .unwrap()
        .take_abilities(crate::object::AbilityList {
            acquisition,
            abilities: crate::copiable_abilities::AbilityDefs::from_static(wall_def.abilities),
            printed: crate::object::PrintedFace::new(index::WALL_OF_SHADOWS, 0),
            token: None,
        });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();

    // Actual responses remove both the granting permanent and activating
    // creature. Stop at each response's completed departure, leaving the
    // announced original on stack for the next real action.
    for source in [grantor, recipient] {
        cast(&mut e, index::UNSUMMON.get());
        target(&mut e, source);
        walk(&mut e, |e| {
            e.state.object(source).unwrap().zone == crate::zone::Zone::Hand
        });
        assert_eq!(
            e.state.object(original).unwrap().zone,
            crate::zone::Zone::Stack
        );
    }
    cast(&mut e, index::VIRTUE_OF_KNOWLEDGE.get());
    if let Pending::ChooseCastMode { options, .. } = e.pending() {
        let mode = options
            .iter()
            .position(|option| matches!(option.kind, crate::choice::CastModeKind::Face(1)))
            .unwrap();
        e.apply(P0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    target(&mut e, original);
    walk(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets {
                reason: crate::choice::TargetPrompt::Retarget { .. },
                ..
            }
        )
    });
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        unreachable!();
    };
    assert!(!options.contains(&shadow));
    assert!(
        !options.contains(&stone),
        "copy retains Wall-only requirement after source departure"
    );
    let alternative = synthetic::permanents(&e, index::WALL_OF_STONE.get())[1];
    assert!(
        options.contains(&alternative),
        "an unprotected Wall remains a legal new target"
    );
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![shadow],
                players: vec![],
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![],
        },
    )
    .unwrap();
    walk(&mut e, empty);
    assert_eq!(
        e.state.object(stone).unwrap().zone,
        crate::zone::Zone::Battlefield,
        "original and copy recheck newly acquired Wall protection using frozen definitions"
    );
}

mod mode_edges;
