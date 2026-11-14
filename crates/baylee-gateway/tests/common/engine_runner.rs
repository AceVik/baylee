use super::*;

/// One engine, for one game, over the real engine link.
///
/// No decision clock: a test that wants a seat to run out of time can say so
/// itself, and a clock running under every other test would only add a way for
/// them to fail on a slow machine. `seed`, where given, replaces the one in
/// the game's preset before the engine reads it.
pub(super) async fn run_engine(
    start: v1::StartEngine,
    presets: tokio::sync::mpsc::UnboundedSender<baylee_core::preset::GamePreset>,
    probe: ProbeEnd,
    seed: Option<u64>,
) {
    let Some(mut ws) = dial(&start.gateway_url).await else {
        return;
    };
    send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::EngineHello(v1::EngineHello {
                game_id: start.game_id.clone(),
                token: start.engine_token.clone(),
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
                pid: 0,
                build: baylee_build::short().to_owned(),
            })),
        },
    )
    .await;
    let mut runner = EngineRunner::new();
    let mut armed = baylee_engine_server::Armed::default();
    let measured_from = tokio::time::Instant::now();
    loop {
        runner.wait_from(u64::try_from(measured_from.elapsed().as_millis()).unwrap());
        let delay = runner
            .entrance_deadline()
            .map(|at| std::time::Duration::from_millis(at.saturating_sub(wall_ms())));
        let now = tokio::time::Instant::now();
        if probe.clocked {
            armed.sync(&runner.clocks(), |clock| {
                now + std::time::Duration::from_secs(u64::from(clock.secs))
            });
        }
        let next = armed.next();
        let out = tokio::select! {
            () = async {
                match next {
                    Some((_, at)) => tokio::time::sleep_until(at).await,
                    None => std::future::pending::<()>().await,
                }
            } => {
                let Some((clock, _)) = next else { continue };
                armed.fired(clock);
                runner.measure_until(u64::try_from(measured_from.elapsed().as_millis()).unwrap());
                runner.tell_time(wall_ms());
                runner.timeout(clock)
            }
            () = async {
                if let Some(delay) = delay { tokio::time::sleep(delay).await; }
                else { std::future::pending::<()>().await; }
            } => {
                runner.measure_until(u64::try_from(measured_from.elapsed().as_millis()).unwrap());
                runner.tell_time(wall_ms());
                runner.finish_entrance()
            }
            msg = next_msg(&mut ws) => {
                runner.measure_until(u64::try_from(measured_from.elapsed().as_millis()).unwrap());
                let Some(mut msg) = msg else { break };
                if matches!(&msg, v1::envelope::Msg::SeatFrame(_)) {
                    probe.seat_frames.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                // A replay job, as `baylee-engine-server` runs one: the
                // tape's pieces, then the socket closes.
                if let v1::envelope::Msg::ProjectRecord(order) = msg {
                    project(&mut ws, order, &probe).await;
                    break;
                }
                if let v1::envelope::Msg::FlushRecord(_) = &msg {
                    probe.flushes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if probe.mute.load(std::sync::atomic::Ordering::SeqCst) {
                        continue;
                    }
                }
                if let (Some(seed), v1::envelope::Msg::GameSetup(setup)) = (seed, &mut msg) {
                    let mut preset: baylee_core::preset::GamePreset =
                        serde_json::from_slice(&setup.preset_json).expect("the gateway's preset");
                    preset.seed = seed;
                    setup.preset_json = serde_json::to_vec(&preset).expect("a preset");
                }
                if let v1::envelope::Msg::GameSetup(setup) = &msg
                    && let Ok(preset) = serde_json::from_slice(&setup.preset_json) {
                    let _ = presets.send(preset);
                }
                runner.tell_time(wall_ms());
                runner.handle(Envelope { msg: Some(msg) }, &[])
            }
        };
        for envelope in out {
            send(&mut ws, &envelope).await;
        }
        // After the frames went out: a test that reads the hash and then
        // asks the gateway something is asking about a game whose every
        // answer is already on the wire.
        probe.hash.send_replace(
            runner
                .session()
                .map(baylee_gamehost::Session::snapshot_hash),
        );
        if runner.finished() {
            break;
        }
    }
}

fn wall_ms() -> u64 {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

/// A replay job, as `baylee-engine-server` runs one: the tape's pieces on
/// the engine link. A muted probe's engine never answers (the `504` test).
async fn project(ws: &mut Socket, order: v1::ProjectRecord, probe: &ProbeEnd) {
    if probe.mute.load(std::sync::atomic::Ordering::SeqCst) {
        std::future::pending::<()>().await;
    }
    let pieces = tokio::task::spawn_blocking(move || {
        let mut pieces = Vec::new();
        baylee_engine_server::projection::run(&order, baylee_build::short(), &mut |piece| {
            pieces.push(piece);
        });
        pieces
    })
    .await
    .expect("the projection runs");
    for piece in pieces {
        send(
            ws,
            &Envelope {
                msg: Some(v1::envelope::Msg::TapeChunk(piece)),
            },
        )
        .await;
    }
}
