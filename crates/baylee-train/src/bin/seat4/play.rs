//! The socket boundary; the recurrent model never reconnects over missing views.
use std::time::Duration;

use anyhow::{Context as _, anyhow, bail};
use baylee_seat::link::SeatLink;
use baylee_train::live4::{LiveReport4, LiveSeat4};
use futures_util::{SinkExt as _, StreamExt as _};
use prost::Message as _;
use tokio_tungstenite::tungstenite::Message;

pub async fn play(
    link: &mut SeatLink,
    core: &mut LiveSeat4,
    audit: &mut Option<super::audit::Writer>,
) -> anyhow::Result<()> {
    let mut socket = link
        .connect()
        .await
        .map_err(|_| anyhow!("seat connection failed before play"))?
        .ok_or_else(|| anyhow!("room closed before connecting"))?;
    let result = async {
        loop {
            let next = match tokio::time::timeout(Duration::from_secs(20), socket.next()).await {
                Ok(next) => next,
                Err(_) if link.room_closed().await => bail!("room closed before a terminal view"),
                Err(_) => continue,
            };
            match next {
                Some(Ok(Message::Binary(bytes))) => {
                    for envelope in core.hear(&bytes)? {
                        let decision = if matches!(
                            envelope.msg,
                            Some(baylee_protocol::v1::envelope::Msg::PlayerAction(_))
                        ) {
                            Some(
                                core.decision_audit()
                                    .context("generated action has no decision audit")?,
                            )
                        } else {
                            None
                        };
                        if let (Some(audit), Some(decision)) = (audit.as_mut(), decision) {
                            audit.decision(decision, false)?;
                        }
                        socket
                            .send(Message::Binary(envelope.encode_to_vec().into()))
                            .await
                            .map_err(|_| {
                                anyhow!(
                                    "seat action send failed; recurrent history cannot continue"
                                )
                            })?;
                        if let (Some(audit), Some(decision)) = (audit.as_mut(), decision) {
                            audit.decision(decision, true)?;
                        }
                    }
                    if core.finished() {
                        return Ok(());
                    }
                }
                Some(Ok(Message::Ping(bytes))) => socket
                    .send(Message::Pong(bytes))
                    .await
                    .map_err(|_| anyhow!("seat heartbeat send failed"))?,
                Some(Ok(Message::Close(_))) | None => {
                    bail!(
                        "seat socket disconnected; refusing to continue with missing observations"
                    );
                }
                Some(Err(_)) => bail!("seat socket failed; recurrent history is incomplete"),
                Some(Ok(_)) => {}
            }
        }
    }
    .await;
    let _ = socket.close(None).await;
    result
}

/// Preserve aggregate counters after cancellation/transport failures too. Error
/// objects are deliberately not accepted: socket URLs may contain spent tickets.
pub fn report_after(report: &LiveReport4, failed: bool) -> LiveReport4 {
    let mut report = report.clone();
    if failed && report.failure.is_none() {
        report.failure =
            Some("seat session failed or was stopped; recurrent play did not complete".into());
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_preserve_counters_and_safe_adapter_reason() {
        let original = LiveReport4 {
            kind: "local_neural_v4",
            observations: 17,
            neural_answers: 3,
            fallback_answers: 2,
            failure: Some("neural observation failed; recurrent trajectory stopped".into()),
            ..LiveReport4::default()
        };
        let saved = report_after(&original, true);
        assert_eq!(saved.observations, 17);
        assert_eq!(saved.neural_answers, 3);
        assert_eq!(saved.fallback_answers, 2);
        assert_eq!(saved.failure, original.failure);
    }

    #[test]
    fn transport_or_operator_failure_gets_a_static_reason_without_raw_error_data() {
        let original = LiveReport4 {
            observations: 9,
            ..LiveReport4::default()
        };
        assert!(report_after(&original, false).failure.is_none());
        let saved = report_after(&original, true);
        assert_eq!(saved.observations, 9);
        assert_eq!(
            saved.failure.as_deref(),
            Some("seat session failed or was stopped; recurrent play did not complete")
        );
        assert!(original.failure.is_none());
    }
}
