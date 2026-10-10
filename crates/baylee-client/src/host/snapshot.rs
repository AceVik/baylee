//! A captured host view, for repeatable renderer profiling without a server.
//! Only enabled in development builds and explicitly selected by environment.

use super::{DuelHost, GameStatic, HostMessage, PlayerAction, PlayerId, PlayerView};

struct SnapshotHost {
    seat: PlayerId,
    messages: Vec<HostMessage>,
}

pub(crate) fn from_env() -> Result<Option<Box<dyn DuelHost>>, String> {
    let Some(path) = std::env::var_os("BAYLEE_DEV_SNAPSHOT") else {
        return Ok(None);
    };
    let bytes = std::fs::read(path).map_err(|e| format!("profiling snapshot: {e}"))?;
    let (statics, view): (GameStatic, PlayerView) =
        serde_json::from_slice(&bytes).map_err(|e| format!("profiling snapshot: {e}"))?;
    if statics.view_version != baylee_view::VIEW_VERSION {
        return Err("profiling snapshot uses a different view version".into());
    }
    Ok(Some(Box::new(SnapshotHost {
        seat: statics.your_seat,
        messages: vec![
            HostMessage::Static(Box::new(statics)),
            HostMessage::View(Box::new(view), None),
            HostMessage::Curtain,
        ],
    })))
}

impl DuelHost for SnapshotHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        std::mem::take(&mut self.messages)
    }

    fn submit(&mut self, _: PlayerAction) {}

    fn ready(&mut self) {}

    fn seat(&self) -> PlayerId {
        self.seat
    }

    fn watching(&self) -> bool {
        true
    }
}
