//! The curtain (#256), driven through `poll_host` and `flush_outbox`: when
//! this seat says it has drawn its table, and what it holds back until the
//! table is open.
//!
//! Through the real message loop rather than against `Duel` alone, because
//! both halves are wiring: a `ready` nobody calls holds every table behind
//! the engine's whole wait, and an outbox that is not held loses the
//! standing ability orders the client sends the moment it has a view.

use super::*;
use baylee_client_core::test_support::ViewBuilder;
use std::sync::{Arc, Mutex};

/// A host the test feeds frame by frame, which writes down what it was told.
struct Stage {
    inbox: Arc<Mutex<Vec<HostMessage>>>,
    told: Arc<Mutex<Vec<&'static str>>>,
}

impl DuelHost for Stage {
    fn poll(&mut self) -> Vec<HostMessage> {
        std::mem::take(&mut *self.inbox.lock().unwrap())
    }
    fn submit(&mut self, _: PlayerAction) {
        self.told.lock().unwrap().push("action");
    }
    fn ready(&mut self) {
        self.told.lock().unwrap().push("ready");
    }
    fn seat(&self) -> PlayerId {
        PlayerId::new(0)
    }
    fn link(&self) -> LinkState {
        LinkState::Up
    }
}

/// An open duel over a [`Stage`], with the two systems under test.
struct Table {
    app: App,
    inbox: Arc<Mutex<Vec<HostMessage>>>,
    told: Arc<Mutex<Vec<&'static str>>>,
}

impl Table {
    fn new() -> Self {
        let inbox = Arc::new(Mutex::new(Vec::new()));
        let told = Arc::new(Mutex::new(Vec::new()));
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
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
            .insert_resource(InstalledHost(Box::new(Stage {
                inbox: inbox.clone(),
                told: told.clone(),
            })))
            .add_systems(Update, (poll_host, flush_outbox).chain());
        // `poll_host` reads nothing in `Closed`, where a phase starts.
        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Opening);
        app.update();
        Self { app, inbox, told }
    }

    /// The host hands these over, and a frame runs.
    fn hear(&mut self, messages: Vec<HostMessage>) {
        self.inbox.lock().unwrap().extend(messages);
        self.app.update();
    }

    fn told(&self) -> Vec<&'static str> {
        self.told.lock().unwrap().clone()
    }

    fn duel(&mut self) -> Mut<'_, Duel> {
        self.app.world_mut().resource_mut::<Duel>()
    }
}

fn view() -> HostMessage {
    HostMessage::View(Box::new(ViewBuilder::new(2).build()))
}

fn statics() -> HostMessage {
    HostMessage::Static(Box::new(baylee_client_core::test_support::statics(2)))
}

/// The seat says it is ready once its first view is built, and once only:
/// every later view, and the curtain, leave it said.
#[test]
fn a_seat_says_it_is_ready_when_its_first_view_is_built() {
    let mut table = Table::new();
    table.hear(vec![statics()]);
    assert_eq!(table.told(), [] as [&str; 0], "no view, nothing drawn");
    table.hear(vec![view()]);
    assert_eq!(table.told(), ["ready"]);
    table.hear(vec![view(), view()]);
    assert_eq!(table.told(), ["ready"], "said once");
    table.hear(vec![HostMessage::Curtain, statics(), view()]);
    assert_eq!(table.told(), ["ready"], "and never again once it is open");
}

/// A seat that attaches again before the table is open opens with the
/// payload again, and the engine may not have heard it: it says it again.
#[test]
fn a_seat_that_attaches_again_before_the_curtain_says_it_again() {
    let mut table = Table::new();
    table.hear(vec![statics(), view()]);
    table.hear(vec![statics(), view()]);
    assert_eq!(table.told(), ["ready", "ready"]);
}

/// Nothing leaves before the table is open, and nothing is lost either:
/// what was queued goes out on the frame the curtain arrives.
#[test]
fn the_outbox_is_held_until_the_curtain_and_sent_with_it() {
    let mut table = Table::new();
    table.hear(vec![statics(), view()]);
    table.duel().submit(PlayerAction::MulliganKeep);
    table.hear(vec![]);
    table.hear(vec![]);
    assert_eq!(table.told(), ["ready"], "sent before the table was open");
    assert_eq!(table.duel().outbox().len(), 1, "and it is still queued");

    table.hear(vec![HostMessage::Curtain]);
    assert_eq!(table.told(), ["ready", "action"]);
    assert!(table.duel().outbox().is_empty());
}

/// The engine keeps a seat's standing orders for the whole game, so none is
/// sent twice (#285): not when a `GameStatic` is re-sent because the seat
/// earned a printing, and not when the seat comes back after a drop, which
/// opens on the same payloads and the curtain.
#[test]
fn a_standing_order_is_sent_once_a_game() {
    use baylee_client_core::automation::{AbilityOrder, set_ability_order};
    let mut table = Table::new();
    let mut prefs = prefs::Prefs::default();
    set_ability_order(
        &mut prefs.edit().ability_orders,
        AbilityOrder {
            ability: baylee_core::ids::AbilityRef::new(baylee_core::ids::CardIndex::new(12), 0),
            pass: true,
            answer: None,
        },
    );
    table
        .app
        .insert_resource(prefs)
        .add_systems(Update, run_autopilot.after(poll_host).before(flush_outbox));
    let shown = || {
        HostMessage::View(Box::new(
            ViewBuilder::new(2)
                .with_battlefield(
                    1,
                    vec![baylee_client_core::test_support::printed(
                        200, 1, "Shown", 12,
                    )],
                )
                .build(),
        ))
    };
    table.hear(vec![statics(), shown()]);
    table.hear(vec![HostMessage::Curtain]);
    assert_eq!(
        table.told(),
        ["ready", "action"],
        "the order, once the table is open"
    );

    table.hear(vec![statics(), shown()]);
    table.hear(vec![]);
    assert_eq!(
        table.told(),
        ["ready", "action"],
        "a print table re-sent mid-game"
    );

    table.hear(vec![statics(), shown(), HostMessage::Curtain]);
    table.hear(vec![]);
    assert_eq!(
        table.told(),
        ["ready", "action"],
        "a reconnect to the same game"
    );
}
