#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// #166 / #170. The pool's first card to put a time counter on anything is
/// a land the agent cannot see as a land.
///
/// Trenzalore Clocktower is `{T}: Add {U}. Put a time counter on Trenzalore
/// Clocktower.`, and #166 was opened on the worry that the counter would be
/// *scored* backwards — a time counter is a delay on a suspended card, and
/// this one is a private count to twelve that its controller wants. It is
/// not scored at all, backwards or otherwise: the `AddCounter` sits inside
/// the mana ability with no target, so nothing ever asks `clock_score` about
/// it. The unit test beside `a_time_counter_delays_the_suspended_card_that_is_about_to_cast`
/// holds the scoring half.
///
/// What the card did was worse and was **#170**: `mana_shape` matches a
/// one-element `[Effect::AddMana { .. }]`, so a mana ability with any second
/// sentence was invisible to the planner. The Clocktower is one of 92 such
/// faces — every painland, every Karoo, every Odyssey filter land, the
/// Talismans and Signets, Ancient Tomb. This was a pinned limitation, and it
/// moved on 08.10.2026 (the owner's painland report): the house AI reads
/// sources through `mana_with_riders`, as the client does, with the rider as
/// a price.
///
/// The first assertion keeps it honest: the engine offers the ability, so a
/// cast is the agent taking the offer.
#[test]
fn a_mana_land_that_also_counts_pays_for_the_spell() {
    let mut offered = false;
    let mut pressed = false;
    let mut cast = false;
    let mut control_cast = false;
    for land in ["Trenzalore Clocktower", "Island"] {
        let preset = position(&["Brainstorm", "Brainstorm"], &[land, land]);
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        let agent = HeuristicAgent::new(AIProfile::EXPERT);
        for seq in 0..100 {
            let Some(seat) = pending_player(engine.pending()) else {
                break;
            };
            let view = asked_view(engine.state(), seat, seq, engine.pending());
            if view.turn > 1 {
                break;
            }
            let ours = |id| {
                view.object(id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index == entry(land).card)
            };
            if let Pending::Priority { legal, .. } = engine.pending()
                && seat == PlayerId::new(0)
                && legal.abilities.iter().any(|(id, _)| ours(*id))
                && land == "Trenzalore Clocktower"
            {
                offered = true;
            }
            let action = match engine.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                _ if seat == PlayerId::new(0) => agent.act(&view, engine.pending()),
                Pending::Priority { .. } => PlayerAction::PassPriority,
                other => panic!("unexpected opposing question: {other:?}"),
            };
            if seat == PlayerId::new(0) {
                match &action {
                    PlayerAction::ActivateManaAbility { source, .. }
                    | PlayerAction::ActivateAbility { source, .. }
                        if ours(*source) && land == "Trenzalore Clocktower" =>
                    {
                        pressed = true;
                    }
                    PlayerAction::CastSpell { .. } if land == "Trenzalore Clocktower" => {
                        cast = true;
                    }
                    PlayerAction::CastSpell { .. } => control_cast = true,
                    _ => {}
                }
            }
            engine
                .apply(seat, action)
                .expect("every planned action is legal");
        }
    }
    assert!(
        control_cast,
        "the control is the measurement: two Islands and two Brainstorms must \
         produce a cast, or this test says nothing about the Clocktower"
    );
    assert!(
        offered,
        "the engine must offer the Clocktower's mana ability, or what follows \
         is a board with nothing to press rather than an agent declining"
    );
    assert!(
        pressed && cast,
        "the Clocktower's {{U}} paid for a Brainstorm: pressed {pressed}, cast {cast}"
    );
}

#[test]
fn a_planned_multicolour_cast_survives_the_mana_choice_round_trip() {
    let preset = position(
        &[
            "Baleful Strix",
            "Loran of the Third Path",
            "Loran of the Third Path",
            "Loran of the Third Path",
            "Loran of the Third Path",
        ],
        &["City of Brass", "Swamp"],
    );
    for profile in [AIProfile::STEADY, AIProfile::SHARP, AIProfile::EXPERT] {
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        let agent = HeuristicAgent::new(profile);
        let mut cast = false;
        for seq in 0..100 {
            let Some(seat) = pending_player(engine.pending()) else {
                break;
            };
            let view = asked_view(engine.state(), seat, seq, engine.pending());
            if view.turn > 1 {
                break;
            }
            cast |= view.battlefield.iter().any(|o| {
                o.card
                    .is_some_and(|c| c.index == entry("Baleful Strix").card)
            });
            let action = match engine.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                _ if seat == PlayerId::new(0) => agent.act(&view, engine.pending()),
                Pending::Priority { .. } => PlayerAction::PassPriority,
                _ => panic!("unexpected opposing question: {:?}", engine.pending()),
            };
            engine
                .apply(seat, action)
                .expect("every planned action is legal");
        }
        assert!(
            cast,
            "{profile:?}: two untapped sources must cast Strix despite the white-heavy hand"
        );
    }
}

/// #168. A permanent that prints a free tap and a priced one is ranked by
/// what the mana **costs**, not only by how much of it there is.
///
/// `policy::sources` collapses one permanent to one `manaplan::Source`,
/// because nothing under `manaplan::plan` keys on `ObjectId` and two entries
/// with one id would let the solver tap the same land twice. So the entry
/// that survives the dedup is the *only* mode the agent will ever use for
/// that permanent — and the key ranked on mana made, then colours reached,
/// and never on the price.
///
/// Two shapes, one per column of that key:
///
/// - **Havenwood Battleground** — `{T}: Add {G}` beside `{T}, Sacrifice this
///   land: Add {G}{G}`. More mana won, so the agent sold the land for a
///   green it already had.
/// - **Spire of Industry** — `{T}: Add {C}` beside `{T}, Pay 1 life: Add one
///   mana of any color`. Equal amounts, five colours against one, so it paid
///   the life even when colourless was the whole of what the plan asked for.
///
/// The assertion is the **ability index** rather than the board, because a
/// board says what survived and an index says which button was pressed: a
/// Havenwood still in play could equally mean the agent never tapped it.
/// `pressed` being non-empty is what rules that out, and it is the assertion
/// that would catch a fixture whose land came in tapped.
#[test]
fn a_permanent_with_a_free_and_a_priced_tap_is_ranked_by_what_it_costs() {
    for (board, spell, land) in [
        (
            vec!["Havenwood Battleground"],
            "Llanowar Elves",
            "Havenwood Battleground",
        ),
        (
            vec!["Spire of Industry", "Mox Opal"],
            "Sol Ring",
            "Spire of Industry",
        ),
    ] {
        let preset = position(&[spell], &board);
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        let agent = HeuristicAgent::new(AIProfile::EXPERT);
        let mut pressed: Vec<u32> = Vec::new();
        let mut cast = false;
        for seq in 0..100 {
            let Some(seat) = pending_player(engine.pending()) else {
                break;
            };
            let view = asked_view(engine.state(), seat, seq, engine.pending());
            if view.turn > 1 {
                break;
            }
            let is_land = |id| {
                view.object(id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index == entry(land).card)
            };
            let action = match engine.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                _ if seat == PlayerId::new(0) => agent.act(&view, engine.pending()),
                Pending::Priority { .. } => PlayerAction::PassPriority,
                other => panic!("unexpected opposing question: {other:?}"),
            };
            if seat == PlayerId::new(0) {
                match &action {
                    PlayerAction::ActivateAbility {
                        source,
                        ability_index,
                    } if is_land(*source) => pressed.push(*ability_index),
                    PlayerAction::CastSpell { .. } => cast = true,
                    _ => {}
                }
            }
            engine
                .apply(seat, action)
                .expect("every planned action is legal");
        }
        assert!(
            cast,
            "{land}: the agent never cast {spell}, so the plan this \
             test is about was never made"
        );
        assert!(
            !pressed.is_empty(),
            "{land}: the agent never tapped it at all, so the ranking below \
             is being read off an empty list"
        );
        assert!(
            pressed.iter().all(|index| *index == 0),
            "{land}: the agent pressed {pressed:?}, and index 0 is the free \
             mode. A priced mode that wins the dedup is the only mode there \
             is for that permanent"
        );
    }
}

/// Seat 0's first turn with `hand` and `board`: whether `spell` reached the
/// battlefield, and whether `land` was ever tapped.
fn first_turn(
    profile: AIProfile,
    hand: &[&str],
    board: &[&str],
    spell: &str,
    land: &str,
) -> (bool, bool) {
    let mut engine = Engine::new(&position(hand, board), RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(profile);
    let is = |o: &baylee_view::PublicObject, name: &str| {
        o.card.is_some_and(|c| c.index == entry(name).card)
    };
    let (mut cast, mut tapped) = (false, false);
    for seq in 0..200 {
        let pending = engine.pending();
        let seat = pending_player(pending).expect("nobody has lost on turn 1");
        let view = asked_view(engine.state(), seat, seq, pending);
        if view.turn > 1 {
            return (cast, tapped);
        }
        cast |= view.battlefield_of(PlayerId::new(0)).any(|o| is(o, spell));
        tapped |= view
            .battlefield_of(PlayerId::new(0))
            .any(|o| is(o, land) && o.status.contains(baylee_view::ObjectStatus::TAPPED));
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ if seat == PlayerId::new(1) => panic!("unexpected opposing question: {pending:?}"),
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        engine
            .apply(seat, action)
            .expect("every planned action is legal");
    }
    panic!("turn 1 never ended");
}

/// #223. Restricted mana (CR 106.6) pays for the spells it names and is
/// left alone for the rest.
///
/// Ancient Ziggurat makes one mana of any colour that may only be spent on a
/// creature spell. The agent read every restricted tap as no source at all,
/// so a creature it could only cast off the Ziggurat stayed in hand. The
/// rest are the ways the fix can go wrong:
///
/// - **The instant.** Its mana would be stranded, so the Ziggurat is not
///   tapped for a Bolt — and with Elves beside the Bolt, a Ziggurat tapped
///   for it in the upkeep is gone by the main phase.
/// - **The order.** Once restricted mana floats, no plan counts it, so a
///   Ziggurat tapped before the Forest leaves a plan one short and a
///   Caryatid in hand. Both board orders are played.
/// - **The other mode.** Abundant Countryside prints `{T}: Add {C}` beside
///   the restricted any-colour tap. The restricted mode takes over that
///   permanent only for a spell it admits, so the {C} still casts a Sol
///   Ring.
///
/// Which colour the Ziggurat's mana is named is `baylee-ai`'s
/// `restricted_mana_is_named_for_a_spell_it_may_pay_for`: in a real hand the
/// creature usually outscores the rest, and the question never goes wrong.
#[test]
fn restricted_mana_pays_for_the_spells_it_names_and_no_others() {
    let zig = "Ancient Ziggurat";
    let fields = "Abundant Countryside";
    let cases: [(&[&str], &[&str], &str); 6] = [
        (&["Llanowar Elves"], &[zig], "Llanowar Elves"),
        (
            &["Lightning Bolt", "Llanowar Elves"],
            &[zig],
            "Llanowar Elves",
        ),
        (&["Sylvan Caryatid"], &[zig, "Forest"], "Sylvan Caryatid"),
        (&["Sylvan Caryatid"], &["Forest", zig], "Sylvan Caryatid"),
        (&["Sol Ring"], &[fields], "Sol Ring"),
        (&["Llanowar Elves"], &[fields], "Llanowar Elves"),
    ];
    for profile in [AIProfile::STEADY, AIProfile::SHARP, AIProfile::EXPERT] {
        for (hand, board, spell) in cases {
            let (cast, _) = first_turn(profile, hand, board, spell, zig);
            assert!(
                cast,
                "{profile:?}: {board:?} did not cast {spell} from {hand:?}"
            );
        }
        let (_, tapped) = first_turn(profile, &["Lightning Bolt"], &[zig], "Lightning Bolt", zig);
        assert!(
            !tapped,
            "{profile:?}: the Ziggurat was tapped for an instant"
        );
    }
}
