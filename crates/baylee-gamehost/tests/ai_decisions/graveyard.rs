#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// #242. A card with flashback in the graveyard is tapped for and cast.
///
/// The engine names a graveyard spell in `legal.castable` only once its cost
/// is floating, and the planner tapped only for what its hand and command
/// zone held, so it never floated that cost: Snapcaster Mage gave Opt
/// flashback, and the turn ended with Opt in the graveyard beside the Island
/// that would have paid for it. The view says what the seat may flash back
/// since `VIEW_VERSION` 30, and the planner walks it.
///
/// Opt and the Mage are cast by hand, so the agent is asked one thing: one
/// untapped land, Opt with flashback, nothing floating. The planner is
/// walked for what to tap and again for the colour to name when the land
/// asks, and the second walk is two, so there are three boards:
///
/// - An Island: what to tap.
/// - A City of Brass: which colour, as the pip count over what could be
///   cast reads it. A count over the hand alone named no blue at all.
///   `CASUAL` is here for this one: below `mulligan_skill` 2 it is the only
///   walk a profile takes, and above it only the fallback.
/// - A City of Brass beside a Blood Rites nothing can pay for: which colour,
///   as the plan reads it. Two red pips outcount Opt's one blue, so the
///   count names red; the plan knows only Opt can be paid for and names
///   blue, if it walks the graveyard. `CASUAL` names red here with #242 or
///   without it, and is left off.
#[test]
fn a_graveyard_card_with_flashback_is_tapped_for_and_cast() {
    let me = PlayerId::new(0);
    let opt_card = entry("Opt").card;
    let profiles = [
        AIProfile::CASUAL,
        AIProfile::STEADY,
        AIProfile::SHARP,
        AIProfile::EXPERT,
    ];
    let boards = profiles
        .into_iter()
        .flat_map(|p| [(p, "Island", None), (p, "City of Brass", None)])
        .chain(
            profiles
                .into_iter()
                .filter(|p| p.mulligan_skill >= 2)
                .map(|p| (p, "City of Brass", Some("Blood Rites"))),
        );
    for (profile, fourth, beside) in boards {
        let lands = ["Island", "Island", "Island", fourth];
        let hand: Vec<&str> = ["Opt", "Snapcaster Mage"]
            .into_iter()
            .chain(beside)
            .collect();
        let preset = position(&hand, &lands);
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        settle(&mut engine, &[]);
        let view = asked_view(engine.state(), me, 1, engine.pending());
        let in_hand = |name: &str| {
            view.hand
                .iter()
                .find(|c| c.card.index == entry(name).card)
                .unwrap_or_else(|| panic!("{name} in hand"))
                .id
        };
        let (opt, mage) = (in_hand("Opt"), in_hand("Snapcaster Mage"));
        float(&mut engine, &untapped(&view, "Island")[..1]);
        engine
            .apply(me, PlayerAction::CastSpell { card: opt })
            .expect("an Island pays for Opt");
        settle(&mut engine, &[]);
        let view = asked_view(engine.state(), me, 2, engine.pending());
        float(&mut engine, &untapped(&view, "Island")[..2]);
        engine
            .apply(me, PlayerAction::CastSpell { card: mage })
            .expect("two Islands pay for the Mage");
        settle(&mut engine, &[opt]);

        let view = asked_view(engine.state(), me, 3, engine.pending());
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected priority, got {:?}", engine.pending());
        };
        assert!(
            view.graveyards[0]
                .iter()
                .any(|o| o.id == opt && o.flashback.is_some())
                && !legal.castable.contains(&opt)
                && untapped(&view, fourth).len() == 1
                && (fourth == "Island" || untapped(&view, "Island").is_empty()),
            "the table must be Opt with flashback, one {fourth} and nothing \
             floating, or what follows is not this question"
        );

        let agent = HeuristicAgent::new(profile);
        for seq in 4..200 {
            let pending = engine.pending();
            let seat = pending_player(pending).expect("nobody has lost on turn 1");
            let view = asked_view(engine.state(), seat, seq, pending);
            if view.turn > 1 {
                break;
            }
            let action = match pending {
                Pending::Priority { .. } if seat != me => PlayerAction::PassPriority,
                _ => agent.act_with_context(&view, pending, &engine.decision_context()),
            };
            engine
                .apply(seat, action)
                .expect("every planned action is legal");
        }
        let view = asked_view(engine.state(), me, 999, engine.pending());
        assert!(
            view.exile
                .iter()
                .flatten()
                .any(|o| o.card.is_some_and(|c| c.index == opt_card)),
            "{profile:?}: Opt stayed in the graveyard with a {fourth} to cast it \
             (beside {beside:?})"
        );
    }
}
