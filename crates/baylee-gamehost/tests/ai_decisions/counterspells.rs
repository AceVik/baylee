#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// Seat 0 holds a Counterspell over two Islands and a Llanowar Elves; seat 1
/// casts `theirs`, in order, in seat 0's main phase, each at the Elves when
/// it targets a creature, and then the agent plays until the stack is empty. Returns the spells seat 1 cast,
/// the targets the agent named for the Counterspell (empty if it never cast
/// it), and whether the Counterspell left the hand.
fn countered(profile: AIProfile, theirs: &[&str]) -> (Vec<ObjectId>, Vec<ObjectId>, bool) {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = position(&["Counterspell"], &["Island", "Island", "Llanowar Elves"]);
    preset.seats[1].starting_hand = Some(theirs.iter().map(|name| entry(name)).collect());
    preset.seats[1].starting_battlefield = ["Plains", "Swamp", "Forest", "Island", "Island"]
        .iter()
        .map(|name| entry(name))
        .collect();
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    settle(&mut engine, &[]);
    let view = asked_view(engine.state(), me, 1, engine.pending());
    let elves = untapped(&view, "Llanowar Elves")[0];
    engine.apply(me, PlayerAction::PassPriority).unwrap();

    let mut cast = Vec::new();
    for name in theirs {
        let view = asked_view(engine.state(), them, 2, engine.pending());
        let spell = view
            .hand
            .iter()
            .find(|c| c.card.index == entry(name).card)
            .expect("their spell in hand")
            .id;
        let lands: &[&str] = match *name {
            "Path to Exile" => &["Plains"],
            "Opt" | "Brainstorm" | "Ancestral Recall" => &["Island"],
            "Dark Ritual" => &["Swamp"],
            _ => &["Swamp", "Forest"],
        };
        for land in lands {
            let view = asked_view(engine.state(), them, 2, engine.pending());
            let source = view
                .battlefield_of(them)
                .find(|o| o.name == *land && !o.status.contains(baylee_view::ObjectStatus::TAPPED))
                .expect("their land")
                .id;
            engine
                .apply(them, PlayerAction::ActivateManaAbility { source })
                .unwrap();
        }
        engine
            .apply(them, PlayerAction::CastSpell { card: spell })
            .expect("their mana pays for it");
        // Pointed at the Elves when it targets a creature, at its own caster
        // when it targets a player (Ancestral Recall asks `ChoosePlayer`), and
        // at nothing when it targets nothing (Opt).
        match engine.pending() {
            Pending::ChooseTargets { options, .. } if options.contains(&elves) => {
                engine
                    .apply(
                        them,
                        PlayerAction::ChooseObjects {
                            objects: vec![elves],
                        },
                    )
                    .expect("pointed at the Elves");
            }
            Pending::ChoosePlayer { .. } => {
                engine
                    .apply(them, PlayerAction::ChoosePlayer(them))
                    .expect("pointed at its caster");
            }
            _ => {}
        }
        cast.push(spell);
    }
    engine.apply(them, PlayerAction::PassPriority).unwrap();

    let agent = HeuristicAgent::new(profile);
    let mut named = Vec::new();
    for seq in 3..100 {
        let pending = engine.pending();
        let seat = pending_player(pending).expect("nobody has lost");
        let view = asked_view(engine.state(), seat, seq, pending);
        if view.stack.is_empty() {
            break;
        }
        let action = match pending {
            Pending::Priority { .. } if seat == them => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        if let (Pending::ChooseTargets { .. }, PlayerAction::ChooseTargets { objects, .. }) =
            (pending, &action)
        {
            named.clone_from(objects);
        }
        engine
            .apply(seat, action)
            .expect("every planned action is legal");
    }
    let view = asked_view(engine.state(), me, 999, engine.pending());
    let spent = !view
        .hand
        .iter()
        .any(|c| c.card.index == entry("Counterspell").card);
    (cast, named, spent)
}

/// #243 with #226's first clause. A counterspell is not spent on a spell
/// that can't be countered.
///
/// Once the engine stopped reading "can't be countered" as "can't be
/// targeted", it offered Counterspell against a lone Abrupt Decay. The
/// agent's counter gate asked only whether an opposing spell was on the
/// stack, so it would have cast it, and the Counterspell would have
/// resolved and countered nothing. Three boards:
///
/// - The Decay alone: the Counterspell stays in hand.
/// - A Path to Exile alone: the control, where the Counterspell is cast, so
///   the first board is not an agent that never counters.
/// - Both, the Decay on top: the Counterspell names the Path, as removal
///   passes over an indestructible creature.
#[test]
fn a_counterspell_is_not_spent_on_a_spell_that_cannot_be_countered() {
    for profile in [AIProfile::STEADY, AIProfile::SHARP, AIProfile::EXPERT] {
        let (_, named, spent) = countered(profile, &["Abrupt Decay"]);
        assert!(
            !spent && named.is_empty(),
            "{profile:?}: a Counterspell was spent on a spell that can't be countered"
        );

        let (cast, named, spent) = countered(profile, &["Path to Exile"]);
        assert!(
            spent && named == cast,
            "{profile:?}: the control: the Path is countered ({named:?} of {cast:?})"
        );

        let (cast, named, spent) = countered(profile, &["Path to Exile", "Abrupt Decay"]);
        assert!(
            spent && named == cast[..1],
            "{profile:?}: with both up, the counter goes to the Path ({named:?} of {cast:?})"
        );
    }
}

/// #226. A counterspell is not spent on a spell that only replaces itself.
///
/// Every hostile spell was worth the same flat bonus to the counter gate, so
/// every named profile countered the first Opt the table cast (measured,
/// with Brainstorm and Fact or Fiction beside it). The boards:
/// - Opt alone, and Brainstorm alone: held.
/// - Ancestral Recall alone: countered. This is the control: a cheap spell
///   is not a cantrip for being cheap, and this one draws three.
/// - Dark Ritual alone, its caster's hand empty behind it: countered. This
///   is a limitation pinned, not a choice: the mana has nothing to pay for,
///   and nothing holds a counter back for a later threat (house-ai.md). It
///   moves when something does.
/// - Opt, then Path to Exile at the Elves: the Path is named.
/// - Ancestral Recall, then Path at the Elves: the Path is named. The two
///   cost the same and neither is a cantrip, so what tells them apart is
///   that the Path is aimed at something of this seat's.
/// - Opt, then Dark Ritual: the Ritual is named. Neither is aimed at
///   anything and both cost one, so what tells them apart is that Opt only
///   replaces itself.
#[test]
fn a_counterspell_is_not_spent_on_a_spell_that_only_replaces_itself() {
    for profile in [AIProfile::STEADY, AIProfile::SHARP, AIProfile::EXPERT] {
        for cantrip in ["Opt", "Brainstorm"] {
            let (_, named, spent) = countered(profile, &[cantrip]);
            assert!(
                !spent && named.is_empty(),
                "{profile:?}: a Counterspell was spent on {cantrip}"
            );
        }

        let (cast, named, spent) = countered(profile, &["Ancestral Recall"]);
        assert!(
            spent && named == cast,
            "{profile:?}: the control: Ancestral Recall is countered ({named:?} of {cast:?})"
        );
        let (cast, named, spent) = countered(profile, &["Dark Ritual"]);
        assert!(
            spent && named == cast,
            "{profile:?}: the pinned limitation moved: a lone Dark Ritual is no longer \
             countered ({named:?} of {cast:?}); update this test and house-ai.md"
        );

        for (first, second) in [
            ("Opt", "Path to Exile"),
            ("Ancestral Recall", "Path to Exile"),
            ("Opt", "Dark Ritual"),
        ] {
            let (cast, named, spent) = countered(profile, &[first, second]);
            assert!(
                spent && named == cast[1..],
                "{profile:?}: with {first} and {second} up, the counter goes to \
                 {second} ({named:?} of {cast:?})"
            );
        }
    }
}
