use super::*;

#[derive(Default)]
struct Evidence {
    prefix: Prefix,
    records: Vec<Vec<u8>>,
}

#[tokio::test]
async fn real_attachment_recovers_committed_prefix_then_seals_verified_terminal_evidence() {
    let game = Uuid::now_v7();
    let mut evidence = Evidence::default();
    // Interrupt after a commit, recover/concede, then recover already terminal.
    for pass in 1..=3 {
        evidence = run(game, pass, evidence).await;
    }
    let config = setup(game, evidence.prefix, 3);
    let mut framed = Vec::new();
    for chunk in evidence.records {
        baylee_worker::frame::write(&mut framed, &chunk, 256 * 1024).unwrap();
    }
    let settlement = baylee_engine_server::tower::verification::verify_record(
        &config.package,
        &config.catalog,
        Binding {
            package: config.package.digest(&config.catalog).unwrap(),
            fence: 3,
            prefix: evidence.prefix,
        },
        framed.as_slice(),
        baylee_gamehost::session::tower::ReplayBudget::default(),
        std::time::Instant::now() + Duration::from_secs(5),
    )
    .unwrap();
    assert_eq!(settlement.outcome, Outcome::Defeat);
}

async fn run(game: Uuid, pass: u64, evidence: Evidence) -> Evidence {
    let config = setup(game, evidence.prefix, pass);
    let digest = config.package.digest(&config.catalog).unwrap();
    let assembly = Assembly::new(
        Lease {
            reservation: Uuid::now_v7(),
            claim: Uuid::now_v7(),
            fence: pass,
        },
        digest,
        config.byte_limit,
        evidence.prefix,
    )
    .unwrap();
    let (socket, worker) = launch(game).await;
    let scenario = Scenario {
        socket,
        game,
        pass,
        evidence,
        digest,
        assembly,
        sealed: false,
        conceded: false,
        reads: 0,
    };
    let story = tokio::spawn(scenario.run(config));
    let result = story.await;
    let joined = tokio::time::timeout(Duration::from_secs(10), worker)
        .await
        .unwrap()
        .unwrap();
    if pass != 1 {
        joined.unwrap();
    }
    result.expect("owned native attachment scenario")
}

struct Scenario {
    socket: WebSocketStream<DuplexStream>,
    game: Uuid,
    pass: u64,
    evidence: Evidence,
    digest: [u8; 32],
    assembly: Assembly,
    sealed: bool,
    conceded: bool,
    reads: u64,
}

impl Scenario {
    async fn run(mut self, config: Setup) -> Evidence {
        assert!(
            matches!(next(&mut self.socket).await.msg, Some(Msg::EngineHello(ref hello)) if hello.game_id == self.game.to_string() && hello.token == "synthetic-token")
        );
        control(
            &mut self.socket,
            Control::Start {
                setup: Box::new(config),
            },
        )
        .await;
        // Input can arrive while initial persistence/reconstruction is pending.
        if self.pass != 3 {
            send(
                &mut self.socket,
                &Envelope {
                    msg: Some(Msg::SeatAttached(v1::SeatAttached {
                        seat: 0,
                        resync: false,
                    })),
                },
            )
            .await;
            seat(&mut self.socket, Msg::SeatReady(v1::SeatReady {})).await;
        }
        loop {
            match next(&mut self.socket).await.msg.unwrap() {
                Msg::TowerFrame(frame) => {
                    let (message, bytes) = Control::decode(frame).unwrap();
                    if self.authority(message, bytes).await {
                        break;
                    }
                }
                Msg::SeatFrame(frame) => {
                    assert_ne!(
                        self.evidence.prefix.chunks, 0,
                        "no view before first whole commit"
                    );
                    let inner = Envelope::decode(frame.envelope).unwrap();
                    if matches!(inner.msg, Some(Msg::ChoiceRequest(_))) && !self.conceded {
                        self.conceded = true;
                        seat(
                            &mut self.socket,
                            Msg::PlayerAction(v1::PlayerActionMsg {
                                game_id: self.game.to_string(),
                                seat_token: String::new(),
                                action: baylee_core::wire::encode(&PlayerAction::Concede),
                            }),
                        )
                        .await;
                    }
                }
                Msg::GameEnded(_) => assert!(self.sealed, "terminal publication waits for seal"),
                Msg::SeatParticipationEnded(_) => {}
                _ => panic!("private evidence leaked through ordinary output"),
            }
        }
        if self.pass != 1 {
            assert!(self.sealed && self.reads != 0);
        }
        self.evidence
    }

    async fn authority(&mut self, message: Control, bytes: Vec<u8>) -> bool {
        match message {
            Control::Read { index } => {
                assert_eq!(index, self.reads);
                self.reads += 1;
                send(
                    &mut self.socket,
                    &Control::Recovery { index }
                        .envelope(self.evidence.records[index as usize].clone())
                        .unwrap(),
                )
                .await;
            }
            Control::Append { .. } => {
                assert_ne!(self.pass, 3, "terminal recovery never adds record bytes");
                if let Some(batch) = self.assembly.push(&message, bytes).unwrap() {
                    assert_eq!(batch.expected, self.evidence.prefix);
                    for data in batch.chunks {
                        self.evidence.prefix = self
                            .evidence
                            .prefix
                            .extended(self.digest, 64 * 1024 * 1024, &data)
                            .unwrap();
                        self.evidence.records.push(data);
                    }
                    let acknowledgement = Acknowledged {
                        fence: self.pass,
                        prefix: self.evidence.prefix,
                    };
                    self.assembly.acknowledge(acknowledgement).unwrap();
                    control(&mut self.socket, Control::Acknowledged { acknowledgement }).await;
                    if self.pass == 1 {
                        return true;
                    }
                }
            }
            Control::Seal { expected } => {
                assert!(self.assembly.can_seal(expected));
                assert_eq!(expected, self.evidence.prefix);
                self.sealed = true;
                control(
                    &mut self.socket,
                    Control::Sealed {
                        acknowledgement: Acknowledged {
                            fence: self.pass,
                            prefix: self.evidence.prefix,
                        },
                    },
                )
                .await;
            }
            Control::Finished {} => {
                assert!(self.sealed);
                return true;
            }
            _ => panic!("wrong engine direction"),
        }
        false
    }
}
