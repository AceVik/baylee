use super::stack::{rules_face, stack_item, stack_text};
use super::zones::{entitled_viewer, shown_elsewhere};
use super::{
    SeatContext, counter, graveyard_price, is_commander, loyalty_now, may_know_card, public_name,
};
use baylee_cards::dsl::AbilityDef;
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
use baylee_engine::choice::Pending;
use baylee_engine::object::{GameObject, ObjectKind, PrintedFace};
use baylee_engine::state::GameState;
use baylee_engine::zone::{Zone, ZoneLocation};
use baylee_view::{CardIdentity, CounterEntry, ObjectStatus, PublicObject, TargetRef};

pub(super) fn exact_object_view_with_access(
    state: &GameState,
    seat: PlayerId,
    source: baylee_core::ids::DamageSourceRef,
    controlled: SeatSet,
) -> Option<baylee_view::DamageSourceView> {
    let viewer = entitled_viewer(state.source_object(source)?, seat, controlled);
    exact_object_view(state, viewer, source)
}

fn exact_object_view(
    state: &GameState,
    seat: PlayerId,
    source: baylee_core::ids::DamageSourceRef,
) -> Option<baylee_view::DamageSourceView> {
    let obj = state.source_object(source)?;
    let known = may_know_card(obj, seat)
        && (!obj.zone.is_hidden_by_default() || (obj.zone == Zone::Hand && obj.owner == seat));
    let chars = obj.characteristics();
    Some(baylee_view::DamageSourceView {
        source,
        name: if known || !obj.zone.is_hidden_by_default() {
            public_name(state, obj, seat)
        } else {
            "Unknown source".into()
        },
        card: obj.card.filter(|_| known).map(|c| CardIdentity {
            index: c.index,
            print: c.print,
            face: obj.face_index,
        }),
        rules: obj.printed_face().filter(|_| known).map(rules_face),
        token: obj
            .token
            .filter(|_| known)
            .map(baylee_cards::tokens::token_id),
        controller: obj.controller,
        zone: match obj.zone {
            Zone::Battlefield => baylee_view::LogZone::Battlefield,
            Zone::Stack => baylee_view::LogZone::Stack,
            Zone::Graveyard => baylee_view::LogZone::Graveyard,
            Zone::Exile => baylee_view::LogZone::Exile,
            Zone::Command => baylee_view::LogZone::Command,
            Zone::Hand => baylee_view::LogZone::Hand,
            Zone::Library | Zone::OutsideGame => baylee_view::LogZone::Library,
        },
        is_current: state
            .object(source.object)
            .is_some_and(|o| o.version == source.version),
        referenced_by: state.source_referenced_by(source),
        colors: if known || !obj.zone.is_hidden_by_default() {
            chars.colors
        } else {
            baylee_core::color::ColorSet::EMPTY
        },
        types: if known || !obj.zone.is_hidden_by_default() {
            chars.types
        } else {
            baylee_core::types::TypeSet::EMPTY
        },
        power: (known || !obj.zone.is_hidden_by_default())
            .then_some(chars.power)
            .flatten(),
        toughness: (known || !obj.zone.is_hidden_by_default())
            .then_some(chars.toughness)
            .flatten(),
        keywords: if known || !obj.zone.is_hidden_by_default() {
            chars.keywords.bits()
        } else {
            0
        },
    })
}

pub(super) fn target_objects(
    state: &GameState,
    seat: PlayerId,
    pending: Option<&Pending>,
    ctx: &SeatContext,
) -> Vec<baylee_view::DamageSourceView> {
    let mut targets = std::collections::BTreeSet::new();
    for grant in state
        .granted_actions
        .iter()
        .filter(|g| g.player == seat || ctx.controlled_players.contains(g.player))
    {
        if let baylee_engine::choice::GrantedActionKind::PreventNextDamage {
            target: TargetRef::Object(reference),
            ..
        } = grant.offer.effect
        {
            targets.insert(reference);
        }
    }
    for &id in state.zones.list(ZoneLocation::Stack) {
        let Some(obj) = state.object(id) else {
            continue;
        };
        for (second, len) in [
            (false, obj.targets.len()),
            (true, obj.second_targets().len()),
        ] {
            for index in 0..len {
                if let Some(reference) = state.recorded_target_reference(
                    id,
                    second,
                    u32::try_from(index).expect("target slot"),
                ) {
                    targets.insert(reference);
                }
            }
        }
    }
    if let Some(Pending::ChooseTargets {
        player,
        reason:
            baylee_engine::choice::TargetPrompt::Retarget {
                current: TargetRef::Object(reference),
                ..
            },
        ..
    }) = pending
        && (*player == seat || ctx.awaiting == Some(seat))
    {
        targets.insert(*reference);
    }
    if let Some(Pending::ChooseNumber {
        reason: baylee_engine::choice::NumberPrompt::TextReplacement { target, .. },
        ..
    }) = pending
    {
        targets.insert(*target);
    }
    targets
        .into_iter()
        .filter_map(|source| {
            exact_object_view_with_access(state, seat, source, ctx.controlled_players)
        })
        .collect()
}

fn object_targets(state: &GameState, obj: &GameObject) -> Vec<TargetRef> {
    let id = obj.id;
    obj.targets
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            state
                .recorded_target_reference(id, false, u32::try_from(index).expect("target slot"))
                .map(TargetRef::Object)
        })
        .chain(obj.target_players.iter().map(TargetRef::Player))
        .chain(
            obj.second_targets()
                .iter()
                .enumerate()
                .filter_map(|(index, _)| {
                    state
                        .recorded_target_reference(
                            id,
                            true,
                            u32::try_from(index).expect("target slot"),
                        )
                        .map(TargetRef::Object)
                }),
        )
        .collect()
}

fn word_changes(state: &GameState, id: ObjectId) -> Vec<baylee_view::WordChange> {
    let Some(object) = state.object(id) else {
        return Vec::new();
    };
    let source = baylee_core::ids::DamageSourceRef {
        object: id,
        version: object.version,
    };
    let map = state.text_changes.get(source);
    let colors = baylee_core::color::Color::ALL
        .iter()
        .enumerate()
        .filter_map(|(from, &color)| {
            let to = map.color_word(color) as u8;
            (usize::from(to) != from).then_some(baylee_view::WordChange {
                basic_land_type: false,
                from: u8::try_from(from).expect("five words"),
                to,
            })
        });
    let lands = baylee_engine::text_changes::BASIC_LAND_TYPES
        .iter()
        .enumerate()
        .filter_map(|(from, &land)| {
            let mapped = map.land_type(land);
            let to = baylee_engine::text_changes::BASIC_LAND_TYPES
                .iter()
                .position(|&word| word == mapped)?;
            (to != from).then_some(baylee_view::WordChange {
                basic_land_type: true,
                from: u8::try_from(from).expect("five words"),
                to: u8::try_from(to).expect("five words"),
            })
        });
    colors.chain(lands).collect()
}

pub(super) fn public_object(
    state: &GameState,
    id: ObjectId,
    seat: PlayerId,
) -> Option<PublicObject> {
    let obj = state.object(id)?;
    let chars = obj.characteristics();
    let known = may_know_card(obj, seat);
    Some(PublicObject {
        word_changes: word_changes(state, id),
        id,
        card: obj.card.filter(|_| known).map(|c| CardIdentity {
            index: c.index,
            print: c.print,
            face: obj.face_index,
        }),
        rules: obj.printed_face().filter(|_| known).map(rules_face),
        name: public_name(state, obj, seat),
        controller: obj.controller,
        owner: obj.owner,
        // Deliberately *not* gated on `known`, unlike the card above: a
        // commander is designated openly (CR 903.3), and `commander_view`
        // hands its identity to the whole table for the same reason. Two
        // structs disagreeing about one fact would be worse than either
        // answer.
        //
        // Manifest is the case that will break this, and gating here would
        // not have fixed it: a commander manifested off a library is a card
        // nobody announced, and `CommanderView::object` names its handle, so
        // blanking one field still leaves it identifiable by cross-reference
        // against the face-down permanent. That needs the handle hidden too,
        // disguise is announced from hand, so this case does not apply.
        commander: is_commander(state, id),
        // The four statuses of CR 110.5: whether a phased-out permanent
        // phased out with what it is attached to is the engine's to know.
        status: ObjectStatus::from_bits(obj.status.public().bits()),
        types: chars.types,
        supertypes: chars.supertypes,
        subtypes: chars.subtypes,
        chosen_subtype: obj.chosen_subtype,
        chosen_opponent: obj.chosen_opponent(),
        chosen_name: obj.chosen_name.map(|named| baylee_view::NamedFace {
            card: named.card(),
            face: named.face(),
        }),
        unlocked_doors: obj
            .doors
            .is_room()
            .then(|| [obj.doors.is_unlocked(0), obj.doors.is_unlocked(1)]),
        suspended: known
            && obj.zone == Zone::Exile
            && obj.riders.contains(&baylee_engine::object::Rider::Suspend)
            && obj.counters.get(baylee_cards_dsl::CounterKind::Time) > 0,
        // The engine holds the definition; the client needs the number, and
        // this crate is the one that can see both.
        token: obj.token.map(baylee_cards::tokens::token_id),
        colors: chars.colors,
        // The projected keywords, and on the stack "can't be countered" as
        // well when only a rider says so: the Cavern of Souls mana the spell
        // was paid with. The bit comes from the predicate the counter itself
        // asks, and only on the stack, since the rider belongs to the spell
        // and not to the permanent it becomes (#243).
        keywords: chars.keywords.bits()
            | if obj.zone == Zone::Stack && !obj.can_be_countered() {
                baylee_cards::dsl::KeywordSet::UNCOUNTERABLE.bits()
            } else {
                0
            },
        power: chars.power,
        toughness: chars.toughness,
        // The base, straight off the object rather than out of the plan:
        // `layers::recompute` starts every projection from exactly this and
        // there is nowhere else the printed body survives. It is the copied
        // card's for a permanent that became a copy, which is what CR 706.2
        // makes true of the object and is the answer a player wants.
        base_power: obj.base.power,
        base_toughness: obj.base.toughness,
        loyalty: loyalty_now(obj, chars.loyalty),
        mana_value: chars.mana_value(),
        damage: obj.damage,
        counters: obj
            .counters
            .iter()
            .map(|(kind, count)| CounterEntry {
                kind: counter(kind),
                count,
            })
            .collect(),
        attached_to: obj.attached_to,
        // `TargetRef` has had a `Player` arm since the view was written and
        // never carried one, because the engine's target list was objects
        // only. A burn spell aimed at a face now says so on the stack.
        //
        // A second instance of the word "target" is drawn on the same list:
        // an arrow is an arrow, and a fight's two creatures are both what the
        // spell points at. Which instance each came from is the engine's
        // business at resolution and nobody's on the table, so the list
        // stays one field and the view keeps its version.
        targets: object_targets(state, obj),
        stack_item: stack_item(obj),
        // Permanents only, because the engine's answer is about a creature
        // *on the battlefield* and a creature card in hand would otherwise
        // come back asleep. The creature test itself is the engine's — one
        // reading of CR 302.6, shared with the attack legality the client
        // is offered, so the drawing and the offer cannot disagree.
        summoning_sick: obj.kind == ObjectKind::Permanent
            && baylee_engine::combat::summoning_sick(state, obj),
        // Permanents only, for the same reason as `summoning_sick`: nothing
        // else can be tapped for it. The engine's own offer reads the grant
        // through the same function, so what the planner is told a land makes
        // is what the engine will hand out when it is tapped.
        granted_mana: (obj.kind == ObjectKind::Permanent)
            .then(|| granted_mana(state, id))
            .flatten(),
        // Permanents only, for the reason the two fields above are: nothing
        // else can be tapped. Not gated on `known` either, and it does not
        // need to be: the abilities are read through the object's own
        // *projected* list, so a permanent that has lost its abilities — the
        // shape anything face-down will have to take — offers none to read.
        board_mana: (obj.kind == ObjectKind::Permanent)
            .then(|| board_mana(state, id))
            .flatten(),
        flashback: graveyard_price(state, id, obj, seat, chars.mana_cost),
        // Permanents only, for `granted_mana`'s reason: the engine offers a
        // granted ability on a permanent and nowhere else, and each entry
        // here is one of those offers.
        grants: if obj.kind == ObjectKind::Permanent {
            grants(state, id, seat)
        } else {
            Vec::new()
        },
    })
}

/// Who granted each of `id`'s granted activated abilities, in the slot order
/// the engine offers them (#212).
///
/// The offer's own walk — `effects::granted_activated`, capped at
/// `GRANTED_SLOTS` — so entry `n` is the ability offered as
/// `granted_ability(n)`, and neither can be renumbered without the other.
///
/// A grantor is named only where this seat may see it: in a zone the view
/// sends ([`shown_elsewhere`]) and face up ([`may_know_card`]). A nontoken
/// card keeps its handle across zones (#240), so the handle of a grantor
/// since returned to a hand or shuffled into a library would say which card
/// in that hidden zone it is. That entry is all `None`, the same as a grant
/// from an emblem or a rule, which has no grantor at all.
fn grants(state: &GameState, id: ObjectId, seat: PlayerId) -> Vec<baylee_view::GrantSource> {
    baylee_engine::effects::granted_activated(state, id)
        .take(baylee_engine::choice::GRANTED_SLOTS as usize)
        .map(|granted| {
            let Some(grantor) = granted
                .source
                .and_then(|source| state.object(source))
                .filter(|o| shown_elsewhere(o, seat) && may_know_card(o, seat))
            else {
                return baylee_view::GrantSource::default();
            };
            let grant = baylee_cards_dsl::Modifier::GrantActivated {
                cost: granted.cost,
                effects: granted.effects,
                mana_ability: granted.mana_ability,
            };
            let home = grant_home(grantor, &grant);
            baylee_view::GrantSource {
                source: Some(grantor.id),
                rules: home.map(|(face, _)| rules_face(face)),
                text: home.and_then(|(face, index)| stack_text(face, index)),
            }
        })
        .collect()
}

/// Which printed face of `grantor` writes `grant`, and which of that face's
/// abilities it is.
///
/// First the abilities the grantor has (its own, or the copied card's),
/// then, only if none of those wrote it, every face of the card it
/// physically is. The second is for a copy clause: Machine God's Effigy
/// copying something has that thing's abilities, and the `…except it has
/// "{T}: Add {U}."` that granted this is printed on the Effigy. It is also
/// the face a transformed card showed when it wrote the grant.
///
/// Both are asked by value ([`baylee_cards::lines::grant_home`]), so a card
/// that wrote nothing equal to it answers `None` and no sentence is
/// guessed: an Opt in a graveyard named as the source of a grant answers
/// `None`, not its scry.
fn grant_home(
    grantor: &GameObject,
    grant: &baylee_cards_dsl::Modifier,
) -> Option<(PrintedFace, u32)> {
    let found = |face: PrintedFace, abilities| {
        let index = baylee_cards::lines::grant_home(abilities, grant)?;
        Some((face, u32::try_from(index).ok()?))
    };
    let list = grantor.printed_ability_list(&crate::session::RegistryLookup);
    let own = list
        .abilities
        .iter()
        .enumerate()
        .find_map(|(index, ability)| {
            if !baylee_cards::lines::grants_in(ability, &mut 0)
                .iter()
                .any(|(_, found)| *found == grant)
            {
                return None;
            }
            let origin = list.origin(index);
            Some((origin.origin?.printed()?, origin.index))
        });
    own.or_else(|| {
        let card = grantor.card?.index;
        let def = baylee_cards::by_index(card)?;
        (0..def.faces.len()).find_map(|face| {
            let printed = PrintedFace::new(card, u8::try_from(face).ok()?)?;
            found(printed, def.abilities_for_face(face))
        })
    })
}

/// The mana a granted ability lets `id` make, when it is one a client's
/// planner can use.
///
/// This crate is the one that can see both halves — the engine's effect table
/// and the DSL that says what an `AddMana` produces — which is the same reason
/// `token` is resolved here. It is not hidden information: the grant comes
/// from a permanent on the battlefield and the ability is already offered in
/// `LegalActions` to whoever may activate it.
/// The **first** grant that is a mana ability, which is the one a planner can
/// use: a permanent may be granted several (Urza's Saga is granted two), and
/// the plan taps it once either way.
fn granted_mana(state: &GameState, id: ObjectId) -> Option<baylee_view::GrantedMana> {
    baylee_engine::effects::granted_activated(state, id)
        .take(baylee_engine::choice::GRANTED_SLOTS as usize)
        .enumerate()
        // `tap_only` and not merely `mana_ability`: a `GrantedMana` has no
        // field for a price, so a grant that charges more than `{T}` —
        // Forgotten Monument sells its Caves a colour for `{T}` and a life —
        // can only be reported as free, and a planner that believed it would
        // tap the land and fail the payment. The ability is still offered in
        // `LegalActions`, so nothing is taken away from the player; what is
        // withheld is a claim the view cannot make honestly.
        .filter(|(_, g)| g.mana_ability && baylee_cards_dsl::tap_only(&g.cost))
        .find_map(|(slot, g)| {
            let mana = baylee_cards_dsl::simple_mana(&g.cost, g.effects)?;
            Some(baylee_view::GrantedMana {
                slot: u32::try_from(slot).unwrap_or(u32::MAX),
                colors: mana.colors,
                amount: mana.amount,
            })
        })
}

/// Which colours a printed mana ability of `id` makes, when the printing does
/// not say.
///
/// The twin of [`granted_mana`] one row along, and the same division of
/// labour: that one covers an ability that is on no card, this one an ability
/// that is on the card and still cannot be read off it. Reflecting Pool,
/// Exotic Orchard and Fellwar Stone read the union of `produced_colors` over
/// the lands of one side of the table; Command Tower, Arcane Signet,
/// Commander's Sphere and Path of Ancestry read a seat's commander identity.
/// This crate is again the one that can see both halves — `mana_shape` says
/// which question the card is asking, `resolve::colors_of` is the engine's
/// own answer to it, and it is the very function that hands the mana out when
/// the permanent is tapped, so what the planner is told cannot drift from
/// what it gets.
///
/// Not hidden information: the permanent is on the battlefield, every land
/// the union reads is too, and a commander is designated openly (CR 903.3).
///
/// The **first** printed ability that needs a board and has an answer, which
/// is the bargain [`granted_mana`] makes for the same reason: a permanent is
/// tapped once, so one is all a plan can spend. No card in the pool prints
/// two, and a permanent that did would have its second refused rather than
/// guessed at — a refusal costs one tap by hand, a wrong answer strands a
/// mana run with the permanent already tapped.
///
/// An ability that currently makes **nothing** is skipped, and if it was the
/// only one the answer is `None`. A Reflecting Pool contributes no colour to
/// the union it reads (CR 106.7, through `Characteristics::produced_colors`),
/// so a lone Pool taps for no colour at all — and "no colours" and "no such
/// ability" are the same answer to every caller: there is no tap here to plan
/// with.
fn board_mana(state: &GameState, id: ObjectId) -> Option<baylee_view::BoardMana> {
    let obj = state.object(id)?;
    // `abilities` and not `by_index`: it is the one accessor that answers for
    // an emblem, a token and a copy, and — the case that matters here — for a
    // permanent whose printed list a continuous effect has replaced.
    obj.abilities(&crate::session::RegistryLookup)
        .iter()
        .enumerate()
        .find_map(|(index, ability)| {
            // `ActivatedConditional` beside `Activated`, because a mana
            // ability with a condition on it is still a mana ability and
            // reading only the first variant is the mistake six readers
            // across this workspace made before it had a name.
            let (AbilityDef::Activated {
                cost,
                effects,
                mana_ability: true,
                ..
            }
            | AbilityDef::ActivatedConditional {
                cost,
                effects,
                mana_ability: true,
                ..
            }) = ability
            else {
                return None;
            };
            let (source, _, _) = baylee_cards_dsl::mana_shape(cost, effects)?;
            // Everything the card can answer alone stays the card's: a Forest
            // and a Birds of Paradise have nothing to gain from a projection
            // and would cost the wire a colour list each.
            if !matches!(
                source,
                baylee_cards_dsl::ManaSource::IntrinsicBasicLandTypes
                    | baylee_cards_dsl::ManaSource::CommanderIdentity
                    | baylee_cards_dsl::ManaSource::LandColor { .. }
                    | baylee_cards_dsl::ManaSource::Chosen
                    | baylee_cards_dsl::ManaSource::ChosenOr(_)
            ) {
                return None;
            }
            // The chosen colour belongs here for the same reason the other
            // two do, and for one more: it is the only one a *client* could
            // not derive even with the whole board in front of it, because
            // the choice is on the object and is printed on no card.
            let colors = baylee_engine::resolve::colors_of(state, obj.controller, source, id);
            (!colors.is_empty()).then(|| baylee_view::BoardMana {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                colors,
            })
        })
}
