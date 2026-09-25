//! Counters and power/toughness effects: counter placement (with
//! Doubling-Season replacement), counter drains, pump/set P/T effects.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

/// An amount as a **signed** P/T modifier.
///
/// The one place a pump's direction is decided, because it was three: the
/// `SetPTFilter`, `PumpFilter` and `PumpTarget` arms below each carried a
/// closure of its own, and each spelled the sign as
/// `matches!(a, Amount::NegX | Amount::NegXFixed(_))` — a positive list over
/// an enum, three times over. `Amount::is_negative` is the one answer to that
/// question and this is the one caller of it here, so a negative amount added
/// to the DSL cannot be read as a bonus by two arms out of three.
///
/// The magnitude is `amount2`'s and is unsigned on purpose. The engine
/// consumes an evaluated amount at eighteen sites and asks the sign at
/// exactly one — this one; the other seventeen are counting cards to draw,
/// tokens to make or life to gain, where a negative number has no meaning to
/// give.
fn signed(a: &Amount, state: &GameState, you: PlayerId, res: &Resolution) -> i16 {
    let v = amount2(a, state, you, res) as i16;
    if a.is_negative() { -v } else { v }
}

/// Executes one counter/P-T effect.
#[allow(clippy::too_many_lines)] // the family is one flat table
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::AddCounter { kind, amount } => {
            let n = amount2(&amount, state, you, res) as u16;
            // "Put a +1/+1 counter on **each of** up to two target
            // creatures": every target, not the first. `this_object` answers
            // `res.targets.first()`, which is the whole truth for a
            // one-subject effect and silently dropped the second of
            // Rishkar, Peema Renegade's two — the only card in the pool that
            // names more than one. `Effect::UntapTarget` has walked
            // `res.targets` since it was written, so this is a sibling
            // reader that asked fewer questions rather than a rule nobody
            // had.
            //
            // The untargeted spelling still falls back to the source, which
            // is what "put a counter on this creature" needs; and a targeted
            // ability whose targets all became illegal resolves with an
            // empty list and puts nothing anywhere, which is the same
            // no-subject answer `this_object` gave by returning `None`.
            if res.targeted {
                for &target in &res.targets {
                    crate::replacement::put_counters(state, target, kind, n);
                }
            } else {
                crate::replacement::put_counters(state, this_object(res)?, kind, n);
            }
            None
        }
        Effect::AddCounterFilter {
            filter,
            kind,
            amount,
        } => {
            let n = amount2(&amount, state, you, res) as u16;
            let objects: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .copied()
                .collect();
            // Per object, not once for the sweep: "a permanent you control"
            // is asked of each permanent the counters land on, so a board
            // holding one of mine and one of theirs gets two answers.
            for id in objects {
                crate::replacement::put_counters(state, id, kind, n);
            }
            None
        }
        Effect::DrainAllCountersIntoSelf => {
            let mut drained: u16 = 0;
            for id in state.zones.list(ZoneLocation::Battlefield).clone() {
                if let Some(obj) = state.object_mut(id) {
                    let held: Vec<_> = obj.counters.iter().collect();
                    for (kind, n) in held {
                        if n > 0 {
                            obj.counters.set(kind, 0);
                            drained = drained.saturating_add(n);
                        }
                    }
                }
            }
            // The drain half above is removal and stays where it is. The
            // placement half is a resolving ability's effect putting
            // counters on a permanent, which is the first case CR 614.16
            // names, so it goes through the door that applies the
            // multiplying replacements — a Thief of Blood under a Doubling
            // Season arrives twice the size. Writing `counters.add` here
            // also skipped the journal entry, so nothing downstream could
            // see the counters land.
            if drained > 0 {
                crate::replacement::put_counters(
                    state,
                    res.source,
                    baylee_cards_dsl::CounterKind::P1P1,
                    drained,
                );
            }
            // Kept beside the door's own invalidation: the drain is a
            // characteristic change too, and it happens whether or not
            // anything is placed afterwards.
            state.invalidate_projections();
            None
        }
        Effect::SetPTFilter {
            filter,
            power,
            toughness,
            duration,
        } => {
            // Read before anything is registered: an ability that said
            // "target" and got none, or whose object is gone, sets the power
            // and toughness of nobody.
            let this = if matches!(filter, baylee_cards_dsl::Filter::This) {
                this_to_affect(state, res)?
            } else {
                res.source
            };
            let p = signed(&power, state, you, res);
            let t = signed(&toughness, state, you, res);
            let ts = state.next_timestamp();
            let modifier = baylee_cards_dsl::Modifier::SetPT(p, t);
            // `This` means the target here, exactly as it does in
            // `CreateContinuousEffect` — the two are written as one
            // sentence on a card ("becomes a creature with power and
            // toughness equal to its mana value") and had to be able to
            // name the same object. Without this arm the only way to
            // write the P/T half was a filter describing the *kind* of
            // permanent, which then set the P/T of every permanent of
            // that kind on every battlefield. Karn's +1 on an artifact
            // land is what found it: the land's mana value is nought, so
            // every noncreature artifact in the game became a 0/0 and
            // was put into a graveyard by the next state-based check.
            let filters = if matches!(filter, baylee_cards_dsl::Filter::This) {
                smallvec::smallvec![crate::effects::EffectFilter::object(state, this)]
            } else {
                super::bound_now(state, filter, &modifier, you, res.source, None)
            };
            for filter in filters {
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: baylee_cards_dsl::Layer::PtSet,
                    timestamp: ts,
                    duration,
                    filter,
                    modifier,
                });
            }
            None
        }
        Effect::PumpFilter {
            filter,
            controlled_by,
            power,
            toughness,
            keywords,
            duration,
        } => {
            let p = signed(&power, state, you, res);
            let t = signed(&toughness, state, you, res);
            // "Creatures target player controls get -2/-2": the seat is
            // read here, where the resolution knows what it chose, and the
            // set is the one it names on the battlefield now — the same
            // moment CR 611.2c fixes it at. A relation naming nobody (a
            // trigger that lost its target) shrinks nothing, which is the
            // whole sentence doing nothing rather than half of it.
            let seats = controlled_by.map(|rel| super::players_of(rel, state, you, res));
            let filters = super::bound_now(
                state,
                filter,
                &baylee_cards_dsl::Modifier::ModifyPT(p, t),
                you,
                res.source,
                seats.as_deref(),
            );
            pump(state, res, you, &filters, (p, t), keywords, duration);
            None
        }
        Effect::PumpTarget {
            power,
            toughness,
            keywords,
            duration,
        } => {
            let p = signed(&power, state, you, res);
            let t = signed(&toughness, state, you, res);
            // Every target, not just the first: a spell that pumps two
            // creatures is one effect per creature, because an
            // `EffectFilter` names exactly one object.
            let filters: SmallVec<[crate::effects::EffectFilter; 4]> = res
                .targets
                .iter()
                .map(|target| crate::effects::EffectFilter::object(state, *target))
                .collect();
            pump(state, res, you, &filters, (p, t), keywords, duration);
            None
        }
        _ => unreachable!("not a counter/P-T effect"),
    }
}

/// Registers one pump: a P/T modifier, and a keyword grant beside it when
/// the effect carries keywords.
///
/// The two are separate `ContinuousEffect`s because they sit in different
/// layers — keywords in layer 6, P/T in 7c (CR 613.1) — and a `Modifier`
/// carries one change. They share a timestamp so nothing can order itself
/// between the halves of a single pump.
fn pump(
    state: &mut GameState,
    res: &Resolution,
    you: baylee_core::ids::PlayerId,
    filters: &[crate::effects::EffectFilter],
    pt: (i16, i16),
    keywords: baylee_cards_dsl::KeywordSet,
    duration: baylee_cards_dsl::Duration,
) {
    let timestamp = state.next_timestamp();
    for filter in filters {
        let mut fx = crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(res.source),
            controller: you,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::PtModify,
            timestamp,
            duration,
            filter: *filter,
            modifier: baylee_cards_dsl::Modifier::ModifyPT(pt.0, pt.1),
        };
        // A pump of +0/+0 with keywords is Rush of Blood's shape, not a bug:
        // register the P/T half only when it moves something.
        if pt != (0, 0) {
            state.effects.register(fx.clone());
        }
        if !keywords.is_empty() {
            fx.layer = baylee_cards_dsl::Layer::Ability;
            fx.modifier = baylee_cards_dsl::Modifier::AddKeyword(keywords);
            state.effects.register(fx);
        }
    }
}

#[cfg(test)]
mod pump_tests {
    #[allow(clippy::wildcard_imports)] // the family vocabulary, as above
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_cards_dsl::{Duration, Filter, KeywordSet, Layer, Modifier, PlayerRel};

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn state() -> GameState {
        GameState::from_preset(&preset(11, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game")
    }

    /// A permanent with a controller and nothing else: what a pump reads is
    /// the battlefield list and the controller, and a printed card would put
    /// four other characteristics in front of the question.
    fn permanent(state: &mut GameState, seat: PlayerId, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(seat, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    }

    /// A resolution of `source`, controlled by seat 0, pointing at
    /// `targets`.
    fn resolution(source: ObjectId, targets: &[ObjectId]) -> Resolution {
        Resolution {
            source,
            on_stack: source,
            controller: me(),
            effects: vec![],
            pc: 0,
            targets: targets.iter().copied().collect(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: !targets.is_empty(),
            mana_ability: false,
            countered_source: None,
            target_lki: None,
        }
    }

    /// `(layer, modifier, timestamp)` of everything registered, in order.
    fn registered(state: &GameState) -> Vec<(Layer, Modifier, u64)> {
        state
            .effects
            .iter()
            .map(|fx| (fx.layer, fx.modifier, fx.timestamp))
            .collect()
    }

    fn pump_target(power: Amount, toughness: Amount, keywords: KeywordSet) -> Effect {
        Effect::PumpTarget {
            power,
            toughness,
            keywords,
            duration: Duration::UntilEndOfTurn,
        }
    }

    /// **One printed sentence, two continuous effects, one timestamp.**
    ///
    /// "Target creature gets +2/+2 and gains flying until end of turn" is a
    /// single pump, and a `Modifier` carries one change — so it has to be
    /// registered twice. CR 613.1 puts the two halves in different layers
    /// and applies them in layer order, keywords (6) before power and
    /// toughness (7c), which is why they cannot be one effect however much
    /// the card reads like one thing.
    ///
    /// The shared timestamp is the half that is invisible until something
    /// else happens in between. Timestamps order effects *within* a layer
    /// (CR 613.7), so two calls to `next_timestamp` here would leave a gap
    /// another effect could be registered into — and the two halves of one
    /// sentence would then be ordered on either side of an effect that
    /// arrived after both of them.
    #[test]
    fn a_pump_with_keywords_is_two_effects_in_two_layers_under_one_timestamp() {
        let mut state = state();
        let bear = permanent(&mut state, me(), "Grizzly Bears");
        let source = permanent(&mut state, me(), "Giant Growth");
        let mut res = resolution(source, &[bear]);

        assert!(state.effects.is_empty(), "nothing is registered yet");
        exec(
            &mut state,
            &mut res,
            pump_target(Amount::Fixed(2), Amount::Fixed(2), KeywordSet::FLYING),
        );

        let fx = registered(&state);
        assert_eq!(fx.len(), 2, "one sentence, two effects");
        assert_eq!(fx[0].0, Layer::PtModify);
        assert_eq!(fx[0].1, Modifier::ModifyPT(2, 2));
        assert_eq!(fx[1].0, Layer::Ability);
        assert_eq!(fx[1].1, Modifier::AddKeyword(KeywordSet::FLYING));
        assert_eq!(
            fx[0].2, fx[1].2,
            "the halves of one pump share a timestamp, so nothing can be \
             ordered between them"
        );

        // And the two are about the creature that was targeted, not about
        // the spell that did the targeting.
        for effect in state.effects.iter() {
            assert!(
                matches!(effect.filter, crate::effects::EffectFilter::ObjectIs(id, _) if id == bear),
                "a pump names its target: {:?}",
                effect.filter
            );
            assert_eq!(effect.controller, me());
            assert_eq!(effect.duration, Duration::UntilEndOfTurn);
        }
    }

    /// **Each half is registered only when it moves something**, which is
    /// what makes Rush of Blood's shape — a `+0/+0` carrying keywords —
    /// something other than a defect.
    ///
    /// The P/T half of that would be a continuous effect in layer 7c adding
    /// nought to a power and nought to a toughness: it changes no
    /// characteristic and still takes a timestamp, so it is a rank in the
    /// layer's ordering that nothing printed asked for. The keyword half of
    /// a plain Giant Growth is the same nothing from the other side.
    #[test]
    fn a_pump_registers_the_halves_that_move_something_and_no_others() {
        let cases: [(Amount, KeywordSet, &[Layer]); 4] = [
            (
                Amount::Fixed(0),
                KeywordSet::FLYING,
                &[Layer::Ability], // Rush of Blood: keywords and no numbers
            ),
            (
                Amount::Fixed(2),
                KeywordSet::EMPTY,
                &[Layer::PtModify], // Giant Growth: numbers and no keywords
            ),
            (
                Amount::Fixed(2),
                KeywordSet::FLYING,
                &[Layer::PtModify, Layer::Ability],
            ),
            (Amount::Fixed(0), KeywordSet::EMPTY, &[]),
        ];

        for (amount, keywords, expected) in cases {
            let mut state = state();
            let bear = permanent(&mut state, me(), "Grizzly Bears");
            let source = permanent(&mut state, me(), "Rush of Blood");
            let mut res = resolution(source, &[bear]);
            exec(&mut state, &mut res, pump_target(amount, amount, keywords));

            let layers: Vec<Layer> = registered(&state).into_iter().map(|f| f.0).collect();
            assert_eq!(
                layers, expected,
                "{amount:?} with {keywords:?} registers the wrong halves"
            );
        }
    }

    /// **The sign is read per half.** `signed` is the one place a pump's
    /// direction is decided, and it is called once for the power and once
    /// for the toughness — a reader that took the direction from one of them
    /// and applied it to both would be right on every symmetric pump in the
    /// pool and wrong on the asymmetric ones, which is a great many cards
    /// reading `+2/-2`.
    ///
    /// The magnitude is unsigned on purpose, so `NegXFixed(2)` is a two that
    /// happens to point downwards rather than a minus two.
    #[test]
    fn each_half_of_a_pump_carries_its_own_direction() {
        for (power, toughness, expected) in [
            (Amount::Fixed(2), Amount::Fixed(2), Modifier::ModifyPT(2, 2)),
            (
                Amount::NegXFixed(2),
                Amount::NegXFixed(2),
                Modifier::ModifyPT(-2, -2),
            ),
            (
                Amount::Fixed(2),
                Amount::NegXFixed(2),
                Modifier::ModifyPT(2, -2),
            ),
            (
                Amount::NegXFixed(3),
                Amount::Fixed(1),
                Modifier::ModifyPT(-3, 1),
            ),
        ] {
            let mut state = state();
            let bear = permanent(&mut state, me(), "Grizzly Bears");
            let source = permanent(&mut state, me(), "the spell");
            let mut res = resolution(source, &[bear]);
            exec(
                &mut state,
                &mut res,
                pump_target(power, toughness, KeywordSet::EMPTY),
            );

            let fx = registered(&state);
            assert_eq!(fx.len(), 1);
            assert_eq!(fx[0].1, expected, "{power:?}/{toughness:?}");
        }
    }

    /// **Every target, not just the first.** An `EffectFilter` names exactly
    /// one object, so a spell that pumps two creatures is two effects — and
    /// a reader that registered against `targets.first()` would leave the
    /// second creature unchanged while the spell resolved and the journal
    /// said it had.
    #[test]
    fn a_pump_is_registered_once_per_target() {
        let mut state = state();
        let first = permanent(&mut state, me(), "Grizzly Bears");
        let second = permanent(&mut state, me(), "Runeclaw Bear");
        let source = permanent(&mut state, me(), "the spell");
        let mut res = resolution(source, &[first, second]);

        exec(
            &mut state,
            &mut res,
            pump_target(Amount::Fixed(1), Amount::Fixed(1), KeywordSet::EMPTY),
        );

        let named: Vec<ObjectId> = state
            .effects
            .iter()
            .map(|fx| match fx.filter {
                crate::effects::EffectFilter::ObjectIs(id, _) => id,
                crate::effects::EffectFilter::Dsl(f) => panic!("a dynamic filter: {f:?}"),
            })
            .collect();
        assert_eq!(named, vec![first, second]);
    }

    /// **A relation that names nobody shrinks nothing**, which is the whole
    /// sentence doing nothing rather than half of it.
    ///
    /// "Creatures target player controls get -2/-2" reads its seat where the
    /// resolution knows what it chose. A trigger that lost its target
    /// chooses no player, and the set is then empty rather than absent — the
    /// difference between shrinking nobody's creatures and shrinking
    /// everybody's, because a `None` seat list is what `PumpFilter` passes
    /// when the sentence names no player at all.
    #[test]
    fn a_pump_at_a_player_nobody_chose_touches_no_creature() {
        static CREATURES: Filter = Filter::CREATURE;

        let mut state = state();
        let mine = permanent(&mut state, me(), "Grizzly Bears");
        let theirs = permanent(&mut state, PlayerId::new(1), "Runeclaw Bear");
        for id in [mine, theirs] {
            state.object_mut(id).expect("just created").base_mut().types =
                baylee_core::types::TypeSet::CREATURE;
        }
        let source = permanent(&mut state, me(), "the trigger");
        let mut res = resolution(source, &[]);
        res.chosen_player = None;

        let shrink = Effect::PumpFilter {
            filter: &CREATURES,
            controlled_by: Some(PlayerRel::Chosen),
            power: Amount::NegXFixed(2),
            toughness: Amount::NegXFixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        };
        exec(&mut state, &mut res, shrink);
        assert!(
            state.effects.is_empty(),
            "no player was chosen, so no creature was named"
        );

        // The counter-evidence: the same sentence with a player behind it
        // reaches that player's creatures and nobody else's.
        res.chosen_player = Some(PlayerId::new(1));
        exec(&mut state, &mut res, shrink);
        let named: Vec<ObjectId> = state
            .effects
            .iter()
            .map(|fx| match fx.filter {
                crate::effects::EffectFilter::ObjectIs(id, _) => id,
                crate::effects::EffectFilter::Dsl(f) => panic!("a dynamic filter: {f:?}"),
            })
            .collect();
        assert_eq!(named, vec![theirs]);
    }
}
