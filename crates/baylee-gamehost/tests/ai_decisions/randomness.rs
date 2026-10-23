#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// #87. The chair a `Session` seats is not keyed to the shuffle.
///
/// `Session::new` handed every AI seat `preset.seed` — the same stream that
/// dealt the hands and shuffled the libraries — and `docs/house-ai.md` wrote
/// it down as the rule. For a heuristic that only breaks ties that is
/// merely untidy; for anything that *samples* it is a leak no seat boundary
/// catches, because nothing crosses one: a sampler drawn from the stream
/// that produced the hidden state is correlated with the answer it is
/// supposed to be guessing at.
///
/// So the question is asked of chairs from games that differ in **nothing but
/// the seed**, over a fixture whose hand and battlefield are written into the
/// preset and therefore do not move with it. Every answer must match. The tie
/// is two identical Brainstorms, which is the shape the noise exists to
/// break — without one the assertion would be satisfied by an agent that has
/// no randomness to leak, which is what the test below holds separately.
#[test]
fn a_session_agent_does_not_inherit_the_seed_that_dealt_the_hands() {
    let base = position(&["Brainstorm", "Brainstorm"], &["Island", "Island"]);
    let seated = |seed: u64| {
        let mut preset = base.clone();
        preset.seed = seed;
        preset.seats[0].controller = SeatController::Ai(AIProfile::EXPERT);
        preset
    };
    let chair = |preset: &GamePreset| match Session::new(preset)
        .expect("the fixture builds a session")
        .seat_kind(PlayerId::new(0))
        .expect("seat zero is at the table")
    {
        SeatKind::Ai(agent) => agent.clone(),
        seat => panic!("seat zero is not an AI chair: {seat:?}"),
    };

    let dealt = seated(41);
    let here = chair(&dealt);
    // One other seed would almost always agree by luck: the noise only
    // decides when two scores are equal, and when it does it still has to
    // land on the other side. The unit test for the same tie sweeps 32
    // seeds for that reason, and so does this.
    let elsewhere: Vec<HeuristicAgent> = (0..32).map(|s| chair(&seated(s))).collect();

    let mut engine = Engine::new(&dealt, RegistryLookup).unwrap();
    let mut asked = 0usize;
    for seq in 0..120 {
        let Some(seat) = pending_player(engine.pending()) else {
            break;
        };
        let view = asked_view(engine.state(), seat, seq, engine.pending());
        if view.turn > 2 {
            break;
        }
        let action = match engine.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            _ if seat == PlayerId::new(0) => {
                let mine = here.act(&view, engine.pending());
                for (seed, chair) in elsewhere.iter().enumerate() {
                    assert_eq!(
                        mine,
                        chair.act(&view, engine.pending()),
                        "the same question, answered differently by a chair from \
                         a game that differs only in the seed its cards were \
                         dealt from (seed {seed}, question {asked}, seq {seq})"
                    );
                }
                asked += 1;
                mine
            }
            Pending::Priority { .. } => PlayerAction::PassPriority,
            // The opponent is a post, not a player: it swings with nothing
            // and blocks with nothing, so the only thing moving between the
            // two runs is the chair under test.
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            },
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers {
                blockers: Vec::new(),
            },
            other => panic!("unexpected opposing question: {other:?}"),
        };
        engine
            .apply(seat, action)
            .expect("every answer came out of the offer");
    }
    assert!(
        asked >= 8,
        "the premise: this fixture has to reach the tie often enough to be \
         able to disagree, and it asked only {asked} questions"
    );
}

/// #87, the other half: the chair still *has* randomness, and it is the
/// table's own.
///
/// The test above is satisfied by an agent with no randomness at all, and by
/// a `describe` that does nothing — every chair would share the
/// no-identifier derivation and agree for that reason instead of the right
/// one. So the same tie is asked of chairs from one preset described under
/// thirty-two different public game identifiers, and they must not all
/// answer alike. That is the counter-test for the assertion above and the
/// only thing that says the identifier reaches the chair at all.
#[test]
fn a_described_table_breaks_its_ties_with_its_own_randomness() {
    let mut preset = position(&["Brainstorm", "Brainstorm"], &["Island", "Island"]);
    preset.seats[0].controller = SeatController::Ai(AIProfile::EXPERT);
    let chair = |game: &str| {
        let mut session = Session::new(&preset).expect("the fixture builds a session");
        session.describe(game.to_string(), Vec::new());
        match session
            .seat_kind(PlayerId::new(0))
            .expect("seat zero is at the table")
        {
            SeatKind::Ai(agent) => agent.clone(),
            seat => panic!("seat zero is not an AI chair: {seat:?}"),
        }
    };
    let tables: Vec<HeuristicAgent> = (0..32).map(|i| chair(&format!("game-{i}"))).collect();

    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let mut differed = false;
    let mut asked = 0usize;
    for seq in 0..120 {
        let Some(seat) = pending_player(engine.pending()) else {
            break;
        };
        let view = asked_view(engine.state(), seat, seq, engine.pending());
        if view.turn > 2 {
            break;
        }
        let action = match engine.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            _ if seat == PlayerId::new(0) => {
                let answers: Vec<PlayerAction> = tables
                    .iter()
                    .map(|t| t.act(&view, engine.pending()))
                    .collect();
                differed |= answers.iter().any(|a| *a != answers[0]);
                asked += 1;
                answers[0].clone()
            }
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            },
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers {
                blockers: Vec::new(),
            },
            other => panic!("unexpected opposing question: {other:?}"),
        };
        engine
            .apply(seat, action)
            .expect("every answer came out of the offer");
    }
    assert!(asked >= 8, "the premise: only {asked} questions were asked");
    assert!(
        differed,
        "thirty-two tables, one tie, and every chair answered it the same \
         way: either the identifier never reaches the chair or the chair has \
         no randomness left for the test above to be about"
    );
}
