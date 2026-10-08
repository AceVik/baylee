//! The three edges a cue is decided on, driven through `poll_host`.
//!
//! [`baylee_client_core::cue`] is tested on its own and says nothing
//! about whether anything ever calls it — "declared but never wired" is a
//! bug this client has shipped before, and a silent sink is exactly the
//! kind of feature nobody notices is unwired. So these go through the
//! real message loop, with a host handing over the real payloads.

use super::*;
use baylee_client_core::cue::Cue;
use baylee_client_core::test_support::ViewBuilder;
use baylee_engine::win::{EndReason, GameResult, Victor};

/// A host that hands over one scripted batch and then nothing.
struct ScriptedHost(Vec<HostMessage>);

impl DuelHost for ScriptedHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        std::mem::take(&mut self.0)
    }
    fn submit(&mut self, _: PlayerAction) {}
    fn ready(&mut self) {}
    fn seat(&self) -> PlayerId {
        PlayerId::new(0)
    }
    fn link(&self) -> LinkState {
        LinkState::Local
    }
}

/// An app with just enough in it to run `poll_host` over a script.
fn table_told(messages: Vec<HostMessage>) -> App {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        // `poll_host` moves the phase on, and a `NextState` written without
        // this is written into a schedule nobody runs.
        .add_plugins(bevy::state::app::StatesPlugin)
        .init_asset::<Image>();
    let textures = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        textures::CardTextures::new(&mut images, 1 << 20)
    };
    app.insert_resource(textures)
        .init_resource::<Duel>()
        .init_state::<DuelPhase>()
        .add_message::<DuelReport>()
        .insert_resource(InstalledHost(Box::new(ScriptedHost(messages))))
        .add_systems(Update, poll_host);
    // `poll_host` returns at once in `Closed`, which is where a `DuelPhase`
    // starts; a table nobody has opened is told nothing.
    app.world_mut()
        .resource_mut::<NextState<DuelPhase>>()
        .set(DuelPhase::Playing);
    app.update();
    app
}

fn cues(app: &App) -> &[baylee_client_core::cue::Beat] {
    app.world().resource::<Duel>().cues.pending()
}

/// Just the names, for the many tests that never cared how many.
fn cue_names(app: &App) -> Vec<Cue> {
    cues(app).iter().map(|beat| beat.cue).collect()
}

fn view_at(life: i32) -> PlayerView {
    let mut view = ViewBuilder::new(2).build();
    view.seats[0].life = life;
    view
}

fn priority() -> Pending {
    Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(baylee_engine::choice::LegalActions::default()),
    }
}

/// Two views a moment apart, and the difference is a sound.
#[test]
fn a_life_total_that_moves_is_heard() {
    let app = table_told(vec![
        HostMessage::View(Box::new(view_at(40)), None),
        HostMessage::View(Box::new(view_at(37)), None),
    ]);
    assert_eq!(cue_names(&app), [Cue::MyLifeLost]);
}

/// …and the first view of a table is not twenty life arriving, which is
/// the ledger's rule reaching all the way out to the message loop.
#[test]
fn the_first_view_of_a_table_is_heard_as_nothing() {
    let app = table_told(vec![HostMessage::View(Box::new(view_at(40)), None)]);
    assert!(cues(&app).is_empty());
}

/// A question addressed to this seat.
#[test]
fn being_asked_something_is_heard() {
    let app = table_told(vec![HostMessage::Choice(Box::new(priority()))]);
    assert_eq!(cue_names(&app), [Cue::YourMove]);
}

/// The same question again is the same question. `pump` hands the acting
/// seat its own question back every time anybody says anything.
#[test]
fn the_same_question_re_sent_is_heard_once() {
    let app = table_told(vec![
        HostMessage::Choice(Box::new(priority())),
        HostMessage::Choice(Box::new(priority())),
        HostMessage::Choice(Box::new(priority())),
    ]);
    assert_eq!(cue_names(&app), [Cue::YourMove]);
}

/// The end of a game, read from the chair that lost it.
#[test]
fn the_end_of_a_game_is_heard_from_the_chair_it_happened_to() {
    let over = Pending::GameOver(GameResult {
        winner: Some(Victor::Player(PlayerId::new(1))),
        reason: EndReason::LastPlayerStanding,
    });
    let app = table_told(vec![HostMessage::Choice(Box::new(over))]);
    assert_eq!(cue_names(&app), [Cue::GameLost]);
}

/// A refusal while there is still a game to refuse something in.
#[test]
fn a_refused_action_is_heard() {
    let app = table_told(vec![HostMessage::Failed(
        "illegal action for your seat".into(),
    )]);
    assert_eq!(cue_names(&app), [Cue::Refused]);
}

/// The counter-test, and the one that matters: the bar stops whole at the
/// end of a game and so does the room. A refusal arriving after the
/// result would be the client objecting to something nobody can still do.
#[test]
fn a_refusal_after_the_result_is_not_heard() {
    let over = Pending::GameOver(GameResult {
        winner: Some(Victor::Player(PlayerId::new(0))),
        reason: EndReason::LastPlayerStanding,
    });
    let app = table_told(vec![
        HostMessage::Choice(Box::new(over)),
        HostMessage::Failed("illegal action for your seat".into()),
    ]);
    assert_eq!(
        cue_names(&app),
        [Cue::GameWon],
        "and no `Refused` beside it"
    );
}

/// A question the client answers for the player inside the same frame is
/// a question the player never saw. This is the whole of why the drain is
/// in `Present` and the answer is in `Sync`.
#[test]
fn a_question_the_standing_orders_answer_is_never_heard() {
    let mut app = table_told(vec![HostMessage::Choice(Box::new(priority()))]);
    assert_eq!(cue_names(&app), [Cue::YourMove], "decided");
    app.world_mut()
        .resource_mut::<Duel>()
        .submit(PlayerAction::PassPriority);
    assert!(cues(&app).is_empty(), "and taken back before the drain");
}

/// WT4 through the duel's own doors: a grant half a second after this
/// seat's own action, with nothing foreign between, is held back and
/// counted; the same grant after an opponent's spell arrived in a view is
/// heard.
#[test]
fn a_grant_after_my_own_action_waits_for_the_table_to_move() {
    let mut duel = Duel::default();
    duel.receive_view(ViewBuilder::new(2).build());
    duel.cues.tell_time(10.0);
    duel.receive_choice(priority());
    assert_eq!(duel.cues.take().len(), 1, "the first grant is heard");
    duel.cues.tell_time(11.0);
    duel.submit(PlayerAction::PassPriority);
    duel.receive_choice(Pending::Priority {
        player: PlayerId::new(1),
        legal: Box::default(),
    });
    duel.cues.tell_time(11.5);
    duel.receive_choice(priority());
    assert!(duel.cues.take().is_empty(), "only my own pass came back");
    assert_eq!(duel.cues.suppressed(), 1);

    duel.cues.tell_time(12.0);
    duel.submit(PlayerAction::PassPriority);
    duel.receive_choice(Pending::Priority {
        player: PlayerId::new(1),
        legal: Box::default(),
    });
    let spell = baylee_client_core::test_support::token(90, 1, "Bolt", 0, 0);
    duel.receive_view(ViewBuilder::new(2).with_stack(vec![spell]).build());
    duel.cues.tell_time(12.5);
    duel.receive_choice(priority());
    assert_eq!(
        duel.cues.take().iter().map(|b| b.cue).collect::<Vec<_>>(),
        [Cue::YourMove],
        "an opponent's spell is news"
    );
}

/// The stream a seat is really sent. A table hands each seat its **own**
/// question only (`Session::rebuild` pushes `pending_for(seat)`, nothing
/// else): this seat never receives the opponent's priority, so nothing but
/// its own answer can tell the cues that the table stopped waiting for it.
/// Measured live in the beta.6 QA: two seats passing through a whole turn
/// rang `YourMove` once, at the game's first question, and
/// `cues_suppressed` stayed 0 through grants 0.5 s apart.
fn my_view_at(step: baylee_view::Step) -> PlayerView {
    let mut view = ViewBuilder::new(2).build();
    view.step = step;
    view
}

/// Answered, the table moves on by somebody else (the step), and the seat
/// is asked again two seconds later: that is news and it is heard.
#[test]
fn being_asked_again_after_answering_is_heard_without_a_foreign_question() {
    let mut duel = Duel::default();
    duel.receive_view(my_view_at(baylee_view::Step::Upkeep));
    duel.cues.tell_time(10.0);
    duel.receive_choice(priority());
    assert_eq!(duel.cues.take().len(), 1, "the first grant is heard");
    duel.cues.tell_time(10.5);
    duel.submit(PlayerAction::PassPriority);
    duel.receive_view(my_view_at(baylee_view::Step::Draw));
    duel.cues.tell_time(12.5);
    duel.receive_choice(priority());
    assert_eq!(
        duel.cues.take().iter().map(|b| b.cue).collect::<Vec<_>>(),
        [Cue::YourMove],
        "the next step's grant is a new question"
    );
    assert_eq!(duel.cues.suppressed(), 0);
}

/// …and a grant that only hands back this seat's own answer, half a second
/// later with nothing moved, is held back and counted (rule 3) — which it
/// can only be once the answer has ended the wait.
#[test]
fn a_grant_handing_back_my_own_answer_is_counted_without_a_foreign_question() {
    let mut duel = Duel::default();
    duel.receive_view(my_view_at(baylee_view::Step::Main));
    duel.cues.tell_time(10.0);
    duel.receive_choice(priority());
    duel.cues.take();
    duel.cues.tell_time(11.0);
    duel.submit(PlayerAction::PassPriority);
    duel.cues.tell_time(11.5);
    duel.receive_choice(priority());
    assert!(duel.cues.take().is_empty(), "only my own pass came back");
    assert_eq!(duel.cues.suppressed(), 1, "and the policy says so");
}
