//! A veteran equips a synthetic reward and a real guest across the table
//! reads it, including after both seats are reclaimed. No operator art is used.
use super::{Account, DeckDraft, Endpoint, Runtime, common, signed, statement};
use central_api::advent::{self, table};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde_json::{Value, json};
use uuid::Uuid;

struct Table {
    port: u16,
    id: String,
    guest: String,
    owner_seat: String,
    guest_seat: String,
}

fn ok(port: u16, path: &str, token: Option<&str>, body: &Value) -> Value {
    let (status, answer) = common::http(port, "POST", path, token, &body.to_string());
    // No secrets from a failed seat exchange in test output.
    assert_eq!(status, 200, "POST {path}");
    serde_json::from_str(&answer).unwrap()
}
fn field(value: &Value, name: &str) -> String {
    value[name].as_str().unwrap().to_owned()
}

impl Table {
    fn start(port: u16, owner: &str, deck: &DeckDraft) -> Self {
        let session = ok(port, "/auth/guest", None, &json!({}));
        let guest = field(&session, "token");
        let (code, terms) = common::http(port, "GET", "/terms", None, "");
        if code == 200 {
            let terms: Value = serde_json::from_str(&terms).unwrap();
            let status = common::http(
                port,
                "POST",
                "/account/terms",
                Some(&guest),
                &json!({"version":terms["version"]}).to_string(),
            )
            .0;
            assert!((200..300).contains(&status));
        }
        let opened = ok(
            port,
            "/lobby/games",
            Some(owner),
            &json!({"mode":"open","submitted_deck":{"name":deck.name,"cards":deck.cards,"sideboard":[],"commanders":[]}}),
        );
        let id = field(&opened, "game_id");
        let guest_deck = ok(
            port,
            "/decks",
            Some(&guest),
            &json!({"name":"Guest leaves","cards":["60 Forest"]}),
        );
        let joined = ok(
            port,
            &format!("/lobby/games/{id}/join"),
            Some(&guest),
            &json!({"deck_id":guest_deck["deck_id"]}),
        );
        for token in [owner, guest.as_str()] {
            let status = common::http(
                port,
                "POST",
                &format!("/lobby/games/{id}/ready"),
                Some(token),
                "{}",
            )
            .0;
            assert_eq!(status, 200);
        }
        assert_eq!(
            common::http(
                port,
                "POST",
                &format!("/lobby/games/{id}/start"),
                Some(owner),
                ""
            )
            .0,
            200
        );
        Self {
            port,
            id,
            guest,
            guest_seat: field(&joined, "seat_token"),
            owner_seat: field(&opened, "seat_token"),
        }
    }
    fn path(&self, part: &str) -> String {
        format!("/games/{}/{part}", self.id)
    }
    fn present(&self, seat: &str, proof: &table::Proof) -> u16 {
        common::http(
            self.port,
            "POST",
            &self.path("event-look"),
            Some(seat),
            &json!({"proof":proof}).to_string(),
        )
        .0
    }
    fn page(
        &self,
        viewer: &str,
        kind: advent::Kind,
        cached: Option<advent::Digest>,
    ) -> (u16, String) {
        common::http(
            self.port,
            "POST",
            &self.path("event-art"),
            Some(viewer),
            &json!({"seat":0,"kind":kind,"offset":0,"cached":cached}).to_string(),
        )
    }
    fn verify_guest(&self, seat: &str, meta: advent::AssetMeta) {
        let (status, roster) =
            common::http(self.port, "GET", &self.path("cosmetics"), Some(seat), "");
        assert_eq!(status, 200);
        assert_eq!(
            serde_json::from_str::<Value>(&roster).unwrap(),
            json!({"0":{"event_sleeve":meta}})
        );
        let client =
            baylee_account::GatewayHttp::development(&format!("http://127.0.0.1:{}", self.port))
                .unwrap();
        let game = Uuid::parse_str(&self.id).unwrap();
        let seats = client.advent_table_looks(game, seat).unwrap();
        assert_eq!(seats[0].sleeve, Some(meta));
        let bytes = client
            .advent_table_asset(game, seat, 0, advent::Kind::Sleeve, &meta, || true)
            .unwrap();
        assert_eq!(bytes.len(), meta.size as usize);
        let (status, body) = self.page(seat, advent::Kind::Sleeve, Some(meta.digest));
        assert_eq!(status, 200);
        assert!(
            matches!(serde_json::from_str(&body).unwrap(),advent::Response::Cached{asset} if asset==meta)
        );
        assert_eq!(
            self.page(seat, advent::Kind::Playmat, Some(meta.digest)).0,
            401
        );
    }
    fn reclaim(&self, session: &str) -> String {
        field(
            &ok(
                self.port,
                &format!("/lobby/games/{}/seat", self.id),
                Some(session),
                &json!({}),
            ),
            "seat_token",
        )
    }
    fn end(&self, rt: &Runtime, owner: &str, guest: &str) {
        use baylee_protocol::v1::{self, Envelope, envelope::Msg};
        rt.block_on(async {
            let (mut socket, mut other) = tokio::join!(
                common::dial_seat(self.port, &self.id, owner),
                common::dial_seat(self.port, &self.id, guest)
            );
            tokio::join!(entrance(&mut socket), entrance(&mut other));
            common::send(
                &mut socket,
                &Envelope {
                    msg: Some(Msg::PlayerAction(v1::PlayerActionMsg {
                        game_id: String::new(),
                        seat_token: String::new(),
                        action: baylee_core::wire::encode(
                            &baylee_engine::choice::PlayerAction::Concede,
                        ),
                    })),
                },
            )
            .await;
            tokio::time::timeout(common::WAIT_BUDGET, async {
                while let Some(msg) = common::next_msg(&mut socket).await {
                    if let Msg::ChoiceRequest(choice) = msg
                        && matches!(
                            baylee_core::wire::decode(&choice.pending),
                            Ok(baylee_engine::choice::Pending::GameOver(_))
                        )
                    {
                        return;
                    }
                }
                panic!("owner socket closed before terminal outcome");
            })
            .await
            .expect("the hosted duel ended after the concession");
        });
    }
}

async fn entrance(socket: &mut common::Socket) {
    use baylee_protocol::v1::{self, Envelope, envelope::Msg};
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(socket).await {
                Some(Msg::GameStatic(_)) => {
                    common::send(
                        socket,
                        &Envelope {
                            msg: Some(Msg::SeatReady(v1::SeatReady {})),
                        },
                    )
                    .await;
                }
                Some(Msg::Curtain(_)) => return,
                Some(_) => {}
                None => panic!("participant disconnected before the shared opening"),
            }
        }
    })
    .await
    .unwrap();
}

fn proof(account: &Account, request: table::Request) -> table::Proof {
    let advent::Response::Table {
        response: table::Response::Grant { proof },
    } = account.advent(advent::Request::Table { request }).unwrap()
    else {
        panic!("expected table invitation")
    };
    proof
}

pub(super) fn exercise(
    rt: &Runtime,
    operator: &DatabaseConnection,
    host: &common::Gateway,
    origin: &str,
    endpoint: &Endpoint,
    account: &Account,
    deck: &DeckDraft,
) {
    rt.block_on(operator.execute_raw(statement(
        "INSERT INTO central_identity.gateways (official,origin,name,version,website)
        VALUES(true,$1,'Synthetic official','0',$2) ON CONFLICT DO NOTHING",
        [origin.into(), format!("{origin}/").into()],
    )))
    .unwrap();
    let agent = rt.block_on(common::attach_agent(host));
    let admission = account.admit(origin).unwrap();
    let table = Table::start(host.port, &admission.token, deck);
    let game = Uuid::parse_str(&table.id).unwrap().into_bytes();
    let advent::Response::Inventory { items, .. } = account
        .advent(advent::Request::Inventory { after: None })
        .unwrap()
    else {
        panic!("owner inventory")
    };
    let meta = items
        .iter()
        .find(|item| Some(item.id) == deck.look.sleeve)
        .unwrap()
        .asset;
    let grant = proof(
        account,
        table::Request::Begin {
            origin: origin.into(),
            game,
            seat: 0,
            look: deck.look,
        },
    );
    assert_eq!(table.present(&table.guest_seat, &grant), 403);
    assert_eq!(table.present(&table.owner_seat, &grant), 204);
    assert_eq!(table.page(&table.guest, advent::Kind::Sleeve, None).0, 403);
    table.verify_guest(&table.guest_seat, meta);
    let new_guest = table.reclaim(&table.guest);
    assert_eq!(
        table
            .page(&table.guest_seat, advent::Kind::Sleeve, Some(meta.digest))
            .0,
        403
    );
    let restored = signed(endpoint);
    let signed_in = restored.admit(origin).unwrap();
    let new_owner = table.reclaim(&signed_in.token);
    let resume = || table::Request::Resume {
        origin: origin.into(),
        game,
        seat: 0,
    };
    let replacement = proof(&restored, resume());
    assert_ne!(grant.digest(), replacement.digest());
    assert_eq!(table.present(&table.owner_seat, &grant), 403);
    assert_eq!(table.present(&new_owner, &replacement), 204);
    table.verify_guest(&new_guest, meta);
    table.end(rt, &new_owner, &new_guest);
    let retired = rt.block_on(async {
        for _ in 0..common::WAIT_TRIES {
            if table
                .page(&new_guest, advent::Kind::Sleeve, Some(meta.digest))
                .0
                == 403
                && matches!(
                    restored.advent(advent::Request::Table { request: resume() }),
                    Ok(advent::Response::Table {
                        response: table::Response::NoLook
                    })
                )
            {
                return true;
            }
            tokio::time::sleep(common::WAIT_STEP).await;
        }
        false
    });
    agent.abort();
    assert!(
        retired,
        "finished game retired both its guest access and the owner's recovery selection"
    );
}
