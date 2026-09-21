//! The effect table: registered continuous effects and their lifetimes.
//!
//! Effects are first-class objects with a source, a layer, a duration, and
//! a filter. Removal is structural: effects with
//! [`Duration::WhileSourceOnBattlefield`] are dropped when their source
//! leaves the battlefield, so anthems remove themselves — card code never
//! has to remember to clean up. The `generation` counter drives the
//! characteristic-projection cache: one integer compare per cache hit.

use baylee_cards_dsl::{Duration, Filter, Layer, Modifier};
use baylee_core::ids::{EffectId, ObjectId, PlayerId};

/// Which objects a continuous effect applies to.
#[derive(Clone, Copy, Debug)]
pub enum EffectFilter {
    /// A declarative DSL filter.
    Dsl(&'static Filter),
    /// Exactly one object (created effects like Giant Growth), named by the
    /// pair that identifies it: its id and the `version` it had when the
    /// effect began.
    ///
    /// **Both halves, because an `ObjectId` alone is not an identity here.**
    /// `object.rs`'s module header states the model — an id is stable for a
    /// whole game and `GameState::move_object` bumps `version` instead
    /// (CR 400.7, "it becomes a new object") — and this was the one
    /// identity-tracking site that never adopted it. With the id alone, a
    /// creature pumped by Giant Growth and then blinked with Ephemerate came
    /// back **still pumped**: the returning permanent really is on the
    /// battlefield and really does carry the same id, so no zone test can
    /// tell it from the object the spell was cast at. The version can.
    ObjectIs(ObjectId, u32),
}

impl EffectFilter {
    /// Names `id` as it stands right now.
    ///
    /// A constructor rather than a written-out pair at each of the nine
    /// registration sites: the second half is not a value any of them has an
    /// opinion about, and a site that wrote the wrong one would be an effect
    /// that quietly reaches nothing.
    ///
    /// An object that is not there cannot be named, and `u32::MAX` is how
    /// this says so — it is a version no object in a finite game reaches,
    /// so the effect applies to nothing rather than to whatever is at that
    /// id. Registering against a missing object is a caller's bug and the
    /// `debug_assert` says which caller.
    #[must_use]
    pub fn object(state: &crate::state::GameState, id: ObjectId) -> Self {
        let object = state.object(id);
        debug_assert!(
            object.is_some(),
            "a continuous effect was registered against {id:?}, which is not in the arena"
        );
        Self::ObjectIs(id, object.map_or(u32::MAX, |o| o.version))
    }

    /// Whether this filter names exactly `obj` — the same object, not merely
    /// the same id.
    ///
    /// One predicate with four readers, for the reason [`applies_to`] gives
    /// about itself: `combat::prevent_from` and `prevent_to` each carried
    /// their own copy of the id compare, and a copy is a chance to answer
    /// the identity question differently from its neighbours.
    #[must_use]
    pub fn names(&self, obj: &crate::object::GameObject) -> bool {
        matches!(self, Self::ObjectIs(id, version)
            if *id == obj.id && *version == obj.version)
    }
}

/// Whether a continuous effect carrying this modifier fixes the set of
/// objects it applies to at the moment it begins (CR 611.2c).
///
/// The rule draws its line at what the effect *does*: one that modifies
/// characteristics or changes control affects the objects that were there
/// when it began and no others, so "all creatures get -2/-2 until end of
/// turn" leaves a creature that arrives afterwards alone. One that does
/// neither — a prevention shield, a rule about what players may do — keeps
/// applying to whatever comes along, which is why this cannot be answered
/// by the effect's `Layer`: the layer says where an effect is applied and
/// every variant here names one, including the modifiers that change no
/// characteristic at all.
///
/// Exhaustive deliberately, the way [`crate::layers`]'s dependency table is:
/// a `_ => false` arm would answer "keeps matching for ever" for every
/// modifier added from here on, which is the direction that is silent —
/// the effect simply goes on catching permanents nobody cast it at.
#[must_use]
pub fn locks_its_set(modifier: &Modifier) -> bool {
    match modifier {
        // Characteristics: types, colors, abilities, P/T — and control,
        // which the rule names beside them.
        Modifier::AddType(_)
        | Modifier::RemoveType(_)
        | Modifier::AddSubtype(_)
        | Modifier::AllCreatureTypes
        | Modifier::AllBasicLandTypes
        | Modifier::AddColor(_)
        | Modifier::SetColor(_)
        | Modifier::AddKeyword(_)
        | Modifier::RemoveKeyword(_)
        | Modifier::LoseKeywords
        | Modifier::ProtectionFrom(_)
        | Modifier::BecomeCopyOf(_)
        | Modifier::GrantsFlashback
        | Modifier::GainControl
        | Modifier::AddTypeIfCountersAtLeast { .. }
        | Modifier::AddKeywordIfCountersAtLeast { .. }
        | Modifier::GrantActivated { .. }
        | Modifier::GrantTriggered { .. }
        | Modifier::ModifyPTPerCount { .. }
        | Modifier::ModifyPT(..)
        | Modifier::SetPT(..)
        | Modifier::SwitchPT => true,
        // Neither: a shield that prevents damage, and the rules a player
        // plays under. Teferi's `SorceriesHaveFlash` is the clearest of
        // them — it is about its controller's spells, and a set of objects
        // fixed at resolution would mean the cards in hand at that moment.
        Modifier::LegendRuleOff
        // Two permissions, and CR 611.2c locks a set only for an effect
        // that changes characteristics or control. These change what their
        // controller may do, so there is no set of objects to lock: a land
        // drawn after Exploration resolved is as playable as one already in
        // hand, and a land milled after Crucible entered is as playable as
        // one already in the graveyard.
        | Modifier::PlayLandsFromGraveyard
        | Modifier::ExtraLandDrops(_)
        | Modifier::CantActivateArtifacts
        | Modifier::OpponentsCastAsSorcery
        | Modifier::PlayersCantLose
        | Modifier::CantLoseLife
        | Modifier::PreventDamageToIt
        | Modifier::PreventDamageFromIt
        | Modifier::OpponentsCantSearch
        | Modifier::NoMaxHandSize
        | Modifier::PlayerHexproof
        | Modifier::SorceriesHaveFlash
        | Modifier::ManaIsAnyColor
        | Modifier::SearchTakeover
        // CR 611.2c locks the set for an effect that changes
        // characteristics or control; this changes a rule, so a permanent
        // that arrives later and matches the filter is kept tapped too.
        | Modifier::DoesNotUntap
        | Modifier::MayChooseNotToUntap => false,
    }
}

/// A registered continuous effect.
#[derive(Clone, Debug)]
pub struct ContinuousEffect {
    /// Effect handle.
    pub id: EffectId,
    /// The permanent/spell/emblem that created this effect.
    pub source: Option<ObjectId>,
    /// The player who controls the effect (for "you"/"opponent" filters).
    pub controller: PlayerId,
    /// The layer it applies in.
    pub layer: Layer,
    /// Registration timestamp (effects ordering within a layer).
    pub timestamp: u64,
    /// Its lifetime.
    pub duration: Duration,
    /// Which objects are affected.
    pub filter: EffectFilter,
    /// What it changes.
    pub modifier: Modifier,
}

/// All currently registered continuous effects.
#[derive(Clone, Debug, Default)]
pub struct EffectTable {
    effects: Vec<ContinuousEffect>,
    next_id: u32,
    /// Bumped on every add/remove — the projection cache key.
    pub generation: u64,
}

impl EffectTable {
    /// Registers an effect; bumps the generation.
    pub fn register(&mut self, mut fx: ContinuousEffect) -> EffectId {
        let id = EffectId::new(self.next_id);
        self.next_id += 1;
        fx.id = id;
        self.effects.push(fx);
        self.generation += 1;
        id
    }

    /// Removes effects matching a predicate; bumps the generation if any.
    pub fn remove_where(&mut self, pred: impl Fn(&ContinuousEffect) -> bool) {
        let before = self.effects.len();
        self.effects.retain(|fx| !pred(fx));
        if self.effects.len() != before {
            self.generation += 1;
        }
    }

    /// All active effects (registration order).
    pub fn iter(&self) -> impl Iterator<Item = &ContinuousEffect> {
        self.effects.iter()
    }

    /// All active effects as a slice, so callers can address them by index.
    ///
    /// `layers::LayerPlan` orders indices rather than references: a plan
    /// holding borrows of this table could not coexist with the `&mut
    /// GameState` that stores the projection results.
    #[must_use]
    pub fn as_slice(&self) -> &[ContinuousEffect] {
        &self.effects
    }

    /// Whether any effect is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Whether an effect from `source` with `ability_index` is registered
    /// (static-ability sync).
    #[must_use]
    pub fn has_source_ability(&self, source: ObjectId, modifier: Modifier) -> bool {
        self.effects
            .iter()
            .any(|fx| fx.source == Some(source) && fx.modifier == modifier)
    }

    /// Number of registered effects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.effects.len()
    }
}

/// The activated ability a continuous effect grants `source`, if any.
///
/// One function with three readers, and that is the point of it existing.
/// [`crate::Engine::legal_actions`] offers this ability under the synthetic
/// index `choice::GRANTED_ABILITY`, `start_granted` runs it when the answer
/// comes back, and the view projects what it makes so a client can plan mana
/// through it. Written out three times, an offer and a projection that
/// disagreed would be a land the planner counts on and the engine refuses.
///
/// Registration order is slot order, and it has to be: the offer numbers the
/// grants it finds and the activation decodes that number back, so the two
/// walks must agree on what "the second one" means. The effect table is
/// append-only within a game, which is what makes the order stable.
pub fn granted_activated(
    state: &crate::state::GameState,
    source: ObjectId,
) -> impl Iterator<Item = GrantedAbility> {
    let obj = state.object(source);
    state.effects.iter().filter_map(move |fx| {
        let obj = obj?;
        let Modifier::GrantActivated {
            cost,
            effects,
            mana_ability,
        } = &fx.modifier
        else {
            return None;
        };
        applies_to(state, fx, obj).then_some(GrantedAbility {
            cost: *cost,
            effects,
            mana_ability: *mana_ability,
        })
    })
}

/// Whether a continuous effect reaches `obj`.
///
/// The one reader of "is this effect about this object", and it was three
/// byte-identical copies before it was a function — in `granted_activated`
/// below, in `eval::protected_from` and in `trigger.rs`'s granted-trigger
/// walk. Three copies of a predicate is three chances for one of them to
/// answer a new [`EffectFilter`] variant differently from its neighbours,
/// and the fourth caller (`progress::untap_step`) is what made writing it
/// out a fourth time the wrong move.
///
/// `fx.source.unwrap_or(obj.id)` is the "this" a filter is resolved
/// against. An effect with no source is an emblem's or a rule's, and there
/// is nothing better to point [`Filter::This`] at than the object being
/// asked about — which is what all three copies already did.
///
/// Not [`crate::layers`]'s `matches_projected`: that one asks the same
/// question of a *projection being built*, where reading a characteristic
/// this effect is about to change is the CR 613.8 dependency problem. Here
/// the projection is finished.
#[must_use]
pub fn applies_to(
    state: &crate::state::GameState,
    fx: &ContinuousEffect,
    obj: &crate::object::GameObject,
) -> bool {
    match &fx.filter {
        EffectFilter::ObjectIs(..) => fx.filter.names(obj),
        EffectFilter::Dsl(filter) => {
            (matches!(
                obj.zone,
                crate::zone::Zone::Battlefield | crate::zone::Zone::Stack
            ) || crate::state::filter_reaches_other_zones(filter))
                && crate::eval::matches(
                    filter,
                    state,
                    obj,
                    fx.controller,
                    fx.source.unwrap_or(obj.id),
                )
        }
    }
}

/// One granted activated ability, as the engine and the view both read it.
#[derive(Clone, Copy, Debug)]
pub struct GrantedAbility {
    /// What activating it costs.
    pub cost: baylee_cards_dsl::cost::Cost,
    /// What it does.
    pub effects: &'static [baylee_cards_dsl::effect::Effect],
    /// Whether it is a mana ability (CR 605.1) and so uses no stack.
    pub mana_ability: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::{CardLookup, GameState};
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_cards_dsl::{Duration, Filter, Layer};
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };
    use std::collections::BTreeSet;

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let entry = DeckEntry {
            card: forest,
            print: baylee_core::ids::PrintRef::new(0),
        };
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: (0..60).map(|_| entry).collect(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 8,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    fn permanent(state: &mut GameState, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    }

    fn effect(source: ObjectId, filter: EffectFilter, modifier: Modifier) -> ContinuousEffect {
        ContinuousEffect {
            id: EffectId::new(0),
            source: Some(source),
            controller: me(),
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::UntilEndOfTurn,
            filter,
            modifier,
        }
    }

    #[test]
    fn issue_38_an_unrelated_cross_zone_effect_does_not_spread_hexproof() {
        let mut state = state();
        let source = permanent(&mut state, "source");
        let library = state.zones.list(ZoneLocation::Library(me()))[0];
        let broad = effect(
            source,
            EffectFilter::Dsl(&Filter::Any),
            Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::HEXPROOF),
        );
        assert!(applies_to(&state, &broad, state.object(source).unwrap()));
        assert!(!applies_to(&state, &broad, state.object(library).unwrap()));
        let explicit = effect(
            source,
            EffectFilter::Dsl(&Filter::InZone(baylee_cards_dsl::ZoneRef::NotBattlefield)),
            Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::HEXPROOF),
        );
        state.effects.register(broad.clone());
        state.effects.register(explicit.clone());
        state.refresh_characteristics();
        assert!(applies_to(
            &state,
            &explicit,
            state.object(library).unwrap()
        ));
        assert!(!applies_to(&state, &broad, state.object(library).unwrap()));
    }

    #[test]
    fn issue_122_effect_target_identity_changes_the_snapshot() {
        let mut a = state();
        let source = permanent(&mut a, "source");
        let other = permanent(&mut a, "other");
        let mut b = a.clone();
        a.effects.register(effect(
            source,
            EffectFilter::ObjectIs(source, 0),
            Modifier::ModifyPT(1, 1),
        ));
        b.effects.register(effect(
            source,
            EffectFilter::ObjectIs(other, 0),
            Modifier::ModifyPT(1, 1),
        ));
        assert_ne!(a.snapshot_hash(), b.snapshot_hash());
        assert_eq!(a.snapshot_hash(), a.clone().snapshot_hash());
        let mut c = state();
        let source = permanent(&mut c, "source");
        let mut d = c.clone();
        c.effects.register(effect(
            source,
            EffectFilter::ObjectIs(source, 0),
            Modifier::ModifyPT(1, 1),
        ));
        d.effects.register(effect(
            source,
            EffectFilter::ObjectIs(source, 1),
            Modifier::ModifyPT(1, 1),
        ));
        assert_ne!(c.snapshot_hash(), d.snapshot_hash());
    }

    #[test]
    fn issue_122_granted_costs_are_part_of_effect_identity() {
        let mut a = state();
        let source = permanent(&mut a, "source");
        let mut b = a.clone();
        a.effects.register(effect(
            source,
            EffectFilter::Dsl(&Filter::Any),
            Modifier::GrantActivated {
                cost: baylee_cards_dsl::Cost::TAP,
                effects: &[],
                mana_ability: false,
            },
        ));
        b.effects.register(effect(
            source,
            EffectFilter::Dsl(&Filter::Any),
            Modifier::GrantActivated {
                cost: baylee_cards_dsl::Cost::FREE,
                effects: &[],
                mana_ability: false,
            },
        ));
        assert_ne!(a.snapshot_hash(), b.snapshot_hash());
    }

    #[test]
    fn issue_118_flashback_exiles_any_stack_departure() {
        for destination in [
            ZoneLocation::Graveyard(me()),
            ZoneLocation::Hand(me()),
            ZoneLocation::Library(me()),
        ] {
            for flashback in [false, true] {
                let mut state = state();
                let name = state.names.intern("spell");
                let spell = state.create_bare(me(), ObjectKind::Spell, name, ZoneLocation::Stack);
                if flashback {
                    state
                        .object_mut(spell)
                        .unwrap()
                        .riders
                        .push(crate::object::Rider::Flashback);
                }
                state
                    .move_object(
                        spell,
                        destination,
                        ZonePosition::Top,
                        crate::event::Cause::Effect,
                    )
                    .unwrap();
                assert_eq!(
                    state.object(spell).unwrap().zone,
                    if flashback {
                        crate::zone::Zone::Exile
                    } else {
                        destination.zone()
                    }
                );
            }
        }
    }

    /// Every `Modifier` there is, so that the two classifications below are
    /// compared over the whole enum rather than over the ones somebody
    /// thought of.
    ///
    /// A list of values and not of answers: what each row is held against
    /// is `Modifier::layer()`, which is a second hand-written classification
    /// of the same thirty-nine variants made for a different reason. The
    /// count is checked against the declaration in `static_ability.rs`
    /// below, because `locks_its_set` is exhaustive and a new variant is a
    /// compile error *there* — the risk here is a variant quietly missing
    /// from the comparison, which is silent.
    fn every_modifier() -> Vec<Modifier> {
        const NOTHING: &[baylee_cards_dsl::Effect] = &[];
        use baylee_cards_dsl::{CounterKind, KeywordSet};
        use baylee_core::color::{Color, ColorSet};
        use baylee_core::ids::SubtypeId;
        use baylee_core::types::TypeSet;
        vec![
            Modifier::BecomeCopyOf(ObjectId::new(1, 0)),
            Modifier::GainControl,
            Modifier::AddType(TypeSet::ARTIFACT),
            Modifier::RemoveType(TypeSet::CREATURE),
            Modifier::AddSubtype(SubtypeId::new(1)),
            Modifier::AllCreatureTypes,
            Modifier::AllBasicLandTypes,
            Modifier::AddTypeIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 8,
                types: TypeSet::CREATURE,
            },
            Modifier::AddColor(ColorSet::of(Color::Red)),
            Modifier::SetColor(ColorSet::EMPTY),
            Modifier::AddKeyword(KeywordSet::FLYING),
            Modifier::RemoveKeyword(KeywordSet::FLYING),
            Modifier::LoseKeywords,
            Modifier::AddKeywordIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 8,
                keywords: KeywordSet::FLYING,
            },
            Modifier::GrantActivated {
                cost: baylee_cards_dsl::Cost::TAP,
                effects: NOTHING,
                mana_ability: true,
            },
            Modifier::GrantTriggered {
                trigger: baylee_cards_dsl::Trigger::ETB,
                effects: NOTHING,
                target: None,
            },
            Modifier::GrantsFlashback,
            Modifier::ProtectionFrom(&Filter::CREATURE),
            Modifier::SetPT(2, 2),
            Modifier::ModifyPT(1, 1),
            Modifier::ModifyPTPerCount {
                filter: &Filter::CREATURE,
                p: 1,
                t: 1,
            },
            Modifier::SwitchPT,
            Modifier::LegendRuleOff,
            Modifier::PlayLandsFromGraveyard,
            Modifier::ExtraLandDrops(2),
            Modifier::CantActivateArtifacts,
            Modifier::OpponentsCastAsSorcery,
            Modifier::PlayersCantLose,
            Modifier::CantLoseLife,
            Modifier::PreventDamageToIt,
            Modifier::PreventDamageFromIt,
            Modifier::OpponentsCantSearch,
            Modifier::NoMaxHandSize,
            Modifier::PlayerHexproof,
            Modifier::SorceriesHaveFlash,
            Modifier::ManaIsAnyColor,
            Modifier::SearchTakeover,
            Modifier::DoesNotUntap,
            Modifier::MayChooseNotToUntap,
        ]
    }

    /// The list above is the enum. Read out of the declaration rather than
    /// counted by hand, because a list that is merely long enough would pass
    /// while missing the variant somebody added and answered for by reflex.
    #[test]
    fn the_list_is_every_modifier_the_dsl_declares() {
        let source = include_str!("../../baylee-cards-dsl/src/static_ability.rs");
        let body = source
            .split_once("pub enum Modifier {")
            .expect("the enum is declared there")
            .1;
        let declared: Vec<&str> = body
            .split_once("\n}\n")
            .expect("and it ends")
            .0
            .lines()
            .filter(|line| line.starts_with("    ") && !line.starts_with("     "))
            .map(str::trim)
            .filter(|line| line.starts_with(|c: char| c.is_ascii_uppercase()))
            .map(|line| line.split([' ', '(', '{', ',']).next().unwrap_or(line))
            .collect();

        let listed: Vec<String> = every_modifier()
            .iter()
            .map(|m| {
                let printed = format!("{m:?}");
                printed
                    .split([' ', '('])
                    .next()
                    .unwrap_or(&printed)
                    .to_owned()
            })
            .collect();

        assert_eq!(
            declared.len(),
            39,
            "read {} variants out of the declaration, which is not the enum",
            declared.len()
        );
        assert_eq!(
            declared.iter().copied().collect::<BTreeSet<_>>(),
            listed.iter().map(String::as_str).collect::<BTreeSet<_>>(),
            "the list and the enum have drifted"
        );
    }

    /// **CR 611.2c and CR 613.1 draw one line, and the two readers of it
    /// agree on thirty-eight of thirty-nine.** The rule locks a set for an
    /// effect that modifies characteristics or changes control; the layer
    /// system puts exactly those effects on a layer of their own and parks
    /// everything else on `Layer::Text`, which
    /// `a_rules_modifying_effect_changes_no_characteristic_and_parks_on_text`
    /// describes as the bucket for "things that genuinely change nothing
    /// about an object". So one predicate is checkable against the other,
    /// and neither was written with the other in mind.
    ///
    /// The exception is [`Modifier::CantActivateArtifacts`], Karn's static.
    /// It answers `Layer::Ability` and locks nothing, and the two readings
    /// are not both right: it adds and removes no ability — the artifact
    /// still has one, and CR 602.5a stops it being activated — so by the
    /// criterion its own layer test states it belongs in the parking bucket
    /// beside `SorceriesHaveFlash`. Pinned rather than moved, because
    /// `layers::apply_modifier` writes no characteristic for it either way
    /// and nothing orders effects that write nothing: the day something
    /// does, this is the test that says where to look.
    #[test]
    fn locking_a_set_and_having_a_layer_are_the_same_question_but_once() {
        let mut disagree = Vec::new();
        for modifier in every_modifier() {
            let locks = locks_its_set(&modifier);
            let characteristic = modifier.layer() != Layer::Text;
            if locks != characteristic {
                disagree.push(format!(
                    "{modifier:?} locks={locks} layer={:?}",
                    modifier.layer()
                ));
            }
        }
        assert_eq!(
            disagree,
            vec!["CantActivateArtifacts locks=false layer=Ability"],
            "the two readings of CR 611.2c have drifted apart"
        );
    }

    /// The counts, so that a change which flips a modifier from one side to
    /// the other is a failure and not a quiet re-balancing: twenty-two
    /// modifiers lock the objects they found, seventeen do not.
    #[test]
    fn twenty_two_modifiers_lock_a_set_and_seventeen_do_not() {
        let locking = every_modifier().iter().filter(|m| locks_its_set(m)).count();
        assert_eq!((locking, 39 - locking), (22, 17));
    }

    /// An `ObjectId` alone is not an identity: an id is stable for a whole
    /// game and a zone change makes the card a new object (CR 400.7), with
    /// `version` as the half that says so. Named by the id alone, a creature
    /// pumped by Giant Growth and then blinked with Ephemerate came back
    /// still pumped.
    #[test]
    fn a_created_effect_names_the_object_it_began_on_and_not_the_id() {
        let mut state = state();
        let bear = permanent(&mut state, "Bear");
        let filter = EffectFilter::object(&state, bear);

        assert!(
            filter.names(state.object(bear).expect("just made it")),
            "the same object it was registered against"
        );

        state
            .move_object(
                bear,
                ZoneLocation::Exile(me()),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("it blinks out");
        state
            .move_object(
                bear,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("and back");

        assert!(
            !filter.names(state.object(bear).expect("still the same id")),
            "it came back as a new object, and the pump does not follow it"
        );
    }

    /// The generation is the projection cache key — a pass is reused while
    /// it has not moved. So a registration bumps it, and a removal that
    /// removed nothing must not: a sweep over a duration that matched no
    /// effect otherwise rebuilds every projection on the board, once per
    /// sweep.
    #[test]
    fn the_generation_moves_when_the_table_does_and_not_otherwise() {
        let mut state = state();
        let bear = permanent(&mut state, "Bear");
        let before = state.effects.generation;

        let first = state.effects.register(effect(
            bear,
            EffectFilter::Dsl(&Filter::CREATURE),
            Modifier::ModifyPT(3, 3),
        ));
        assert!(state.effects.generation > before);
        assert_eq!(state.effects.len(), 1);
        assert!(!state.effects.is_empty());

        let after_one = state.effects.generation;
        state
            .effects
            .remove_where(|fx| fx.duration == Duration::UntilYourNextTurn);
        assert_eq!(
            state.effects.generation, after_one,
            "a sweep that matched nothing invalidates nothing"
        );

        let second = state.effects.register(effect(
            bear,
            EffectFilter::Dsl(&Filter::CREATURE),
            Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::FLYING),
        ));
        assert_ne!(first, second, "each registration gets its own handle");
        assert_eq!(state.effects.as_slice()[0].id, first, "registration order");
        assert_eq!(state.effects.as_slice()[1].id, second);
        assert!(
            state
                .effects
                .has_source_ability(bear, Modifier::ModifyPT(3, 3))
        );
        assert!(
            !state
                .effects
                .has_source_ability(bear, Modifier::ModifyPT(1, 1))
        );

        let after_two = state.effects.generation;
        state
            .effects
            .remove_where(|fx| fx.duration == Duration::UntilEndOfTurn);
        assert!(
            state.effects.generation > after_two,
            "and one that did, does"
        );
        assert!(state.effects.is_empty());
    }

    /// CR 613.1 makes the layer a function of the modifier, and this table
    /// stores the layer beside it. A pump registered on the ability layer
    /// would be applied after the P/T it is supposed to modify, so the two
    /// halves of one printed sentence are ordered by what they *are* rather
    /// than by the order they were written in.
    #[test]
    fn a_registered_effect_keeps_the_layer_its_modifier_derives() {
        let mut state = state();
        let bear = permanent(&mut state, "Bear");
        state.effects.register(effect(
            bear,
            EffectFilter::object(&state, bear),
            Modifier::ModifyPT(2, 2),
        ));
        state.effects.register(effect(
            bear,
            EffectFilter::object(&state, bear),
            Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::TRAMPLE),
        ));
        assert_eq!(state.effects.as_slice()[0].layer, Layer::PtModify);
        assert_eq!(state.effects.as_slice()[1].layer, Layer::Ability);
    }

    /// One walk answers "which ability does this permanent have granted to
    /// it", because the offer numbers what it finds and the activation
    /// decodes that number back. Registration order is therefore slot
    /// order, and a grant whose filter does not reach the object is not a
    /// slot at all — for both readers alike, which is the only reason
    /// skipping one is safe.
    #[test]
    fn granted_abilities_come_out_in_registration_order() {
        let mut state = state();
        let bear = permanent(&mut state, "Bear");
        let other = permanent(&mut state, "Other");
        let grant = |mana_ability| Modifier::GrantActivated {
            cost: baylee_cards_dsl::Cost::TAP,
            effects: &[],
            mana_ability,
        };
        // Reaches the Bear, then one that does not, then the Bear again.
        state.effects.register(effect(
            bear,
            EffectFilter::object(&state, bear),
            grant(true),
        ));
        state.effects.register(effect(
            other,
            EffectFilter::object(&state, other),
            grant(false),
        ));
        state.effects.register(effect(
            bear,
            EffectFilter::object(&state, bear),
            grant(false),
        ));

        let found: Vec<bool> = granted_activated(&state, bear)
            .map(|g| g.mana_ability)
            .collect();
        assert_eq!(
            found,
            vec![true, false],
            "the one aimed at another permanent is not this permanent's \
             second slot"
        );
        assert_eq!(granted_activated(&state, other).count(), 1);
    }
}
