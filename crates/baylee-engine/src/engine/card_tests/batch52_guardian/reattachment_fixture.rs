//! Transparent self-reattachment ability exercises the shared attachment gate.
//! These fixtures exercise engine actions; they do not implement the rules under test.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::synthetic::{self, SyntheticLookup};
use baylee_cards_dsl::{AbilityDef, CardDef, Cost, Effect, Filter, TargetSpec};
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const AURA: u32 = 98_721;
static REATTACH_EFFECTS: &[Effect] = &[Effect::AttachSelf {
    target: TargetSpec::Object(&Filter::ARTIFACT),
}];
static REATTACH_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
    Cost::FREE,
    REATTACH_EFFECTS,
    target = Some(TargetSpec::Object(&Filter::ARTIFACT))
)];
fn fixture(
    index_value: u32,
    original: CardIndex,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    let original = baylee_cards::by_index(original).unwrap();
    let face = baylee_cards_dsl::FaceDef {
        mana_cost: baylee_core::mana::ManaCost::parse("{0}"),
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
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
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
fn guardian_existing_aura_move_is_blocked_then_allowed_while_tapped_and_old_attachment_remains() {
    // Add exactly a free self-reattachment activated ability to a real Aura's
    // definition. Enchantment Alteration is Partial, so it is not a helper.
    let real = baylee_cards::by_index(index::POWER_ARTIFACT).unwrap();
    let mut abilities = real.abilities.to_vec();
    abilities.extend_from_slice(REATTACH_ABILITIES);
    let def = fixture(
        AURA,
        index::POWER_ARTIFACT,
        Box::leak(abilities.into_boxed_slice()),
    );
    let mut e = setup(
        def,
        &[
            AURA,
            index::GUARDIAN_BEAST.get(),
            index::TWIDDLE.get(),
            index::TWIDDLE.get(),
        ],
        &[index::SOL_RING.get(), index::SOL_RING.get()],
    );
    let rings = synthetic::permanents(&e, index::SOL_RING.get());
    let (first, second) = (rings[0], rings[1]);
    cast(&mut e, AURA);
    target(&mut e, first);
    walk(&mut e, empty);
    let aura = object(&e, AURA);
    cast(&mut e, index::GUARDIAN_BEAST.get());
    walk(&mut e, empty);
    let beast = object(&e, index::GUARDIAN_BEAST.get());
    assert_eq!(e.state.object(aura).unwrap().attached_to, Some(first));
    priority(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: aura,
            ability_index: 2,
        },
    )
    .unwrap();
    // An activated ability targeting an artifact is not an Aura spell's enchant
    // target: it may target the protected artifact, but attachment must fail.
    target(&mut e, second);
    walk(&mut e, empty);
    assert_eq!(e.state.object(aura).unwrap().attached_to, Some(first));
    cast(&mut e, index::TWIDDLE.get());
    target(&mut e, beast);
    walk(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    walk(&mut e, empty);
    priority(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: aura,
            ability_index: 2,
        },
    )
    .unwrap();
    target(&mut e, second);
    walk(&mut e, empty);
    assert_eq!(e.state.object(aura).unwrap().attached_to, Some(second));
    cast(&mut e, index::TWIDDLE.get());
    target(&mut e, beast);
    walk(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    walk(&mut e, empty);
    assert_eq!(
        e.state.object(aura).unwrap().attached_to,
        Some(second),
        "newly untapped Beast must keep existing Aura"
    );
    priority(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: aura,
            ability_index: 2,
        },
    )
    .unwrap();
    target(&mut e, first);
    walk(&mut e, empty);
    assert_eq!(e.state.object(aura).unwrap().attached_to, Some(second));
    let timestamp = e.state.object(aura).unwrap().timestamp;
    priority(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: aura,
            ability_index: 2,
        },
    )
    .unwrap();
    target(&mut e, second);
    walk(&mut e, empty);
    assert_eq!(
        e.state.object(aura).unwrap().timestamp,
        timestamp,
        "same-host no-op is not a new attachment"
    );
}
