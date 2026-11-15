use super::*;
use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1;
use baylee_tower::{
    cards::{Card, Catalog, Stack},
    v2::{
        execution::Prefix,
        package::{Build, Cohort, Origin, Package},
        run::{Binding, Outcome},
        verification::Resources,
    },
};
use baylee_tower_api::{
    engine::{Assembly, Setup},
    host::{Acknowledged, Lease},
};
use futures_util::StreamExt as _;
use prost::Message as _;
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::protocol::Role;
use uuid::Uuid;

fn setup(game: Uuid, prefix: Prefix, fence: u64) -> Setup {
    let card = baylee_cards::decks::by_name("Forest").unwrap();
    let catalog = Catalog {
        cards: vec![Card {
            card,
            price: 0,
            basic_land: true,
            run: None,
        }],
    };
    let build = Build {
        formation: None,
        deck: vec![Stack { card, copies: 40 }],
        active: Vec::new(),
        potions: [0; 2],
    };
    let package = Package {
        schema: 2,
        cohort: Cohort::new(baylee_engine_server::variant::identity(), &catalog).unwrap(),
        stage: 1,
        opponent: Origin::House { archetype: 1 },
        builds: [build.clone(), build],
        shuffle_seed: 17,
        policy_seed: 19,
    };
    Setup {
        game,
        package,
        catalog,
        fence,
        prefix,
        byte_limit: 64 * 1024 * 1024,
        names: ["Synthetic player".into(), "Synthetic echo".into()],
        resources: Resources::default(),
    }
}

async fn launch(
    game: Uuid,
) -> (
    WebSocketStream<DuplexStream>,
    tokio::task::JoinHandle<Result<(), Failure>>,
) {
    let (client, host) = tokio::io::duplex(64 * 1024);
    let client = WebSocketStream::from_raw_socket(
        client,
        Role::Client,
        Some(super::super::super::ws_config()),
    )
    .await;
    let host = WebSocketStream::from_raw_socket(
        host,
        Role::Server,
        Some(super::super::super::ws_config()),
    )
    .await;
    let worker = tokio::spawn(play(
        client,
        Attach {
            url: "synthetic".into(),
            game_id: game.to_string(),
            token: "synthetic-token".into(),
        },
    ));
    (host, worker)
}

async fn next(socket: &mut WebSocketStream<DuplexStream>) -> Envelope {
    loop {
        let frame = tokio::time::timeout(Duration::from_secs(10), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if frame.is_binary() {
            return Envelope::decode(frame.into_data()).unwrap();
        }
        assert!(
            !frame.is_close(),
            "engine remains live until expected completion"
        );
    }
}

async fn send(socket: &mut WebSocketStream<DuplexStream>, envelope: &Envelope) {
    super::super::send(socket, envelope).await.unwrap();
}

async fn control(socket: &mut WebSocketStream<DuplexStream>, control: Control) {
    send(socket, &control.envelope(Vec::new()).unwrap()).await;
}

async fn seat(socket: &mut WebSocketStream<DuplexStream>, msg: Msg) {
    send(
        socket,
        &Envelope {
            msg: Some(Msg::SeatFrame(v1::SeatFrame {
                seat: 0,
                envelope: Envelope { msg: Some(msg) }.encode_to_vec().into(),
            })),
        },
    )
    .await;
}

mod refusals;
mod replay;
