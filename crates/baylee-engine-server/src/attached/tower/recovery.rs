use super::{Connection, Control, Failure, Progress};
use baylee_engine_server::tower::durable::{Recovery, Table};
use baylee_gamehost::session::tower::ReplayBudget;
use baylee_tower::v2::run::Binding;
use baylee_tower_api::engine::Setup;
use std::{
    io::{self, Cursor, Read},
    sync::mpsc::{Receiver, sync_channel},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc::UnboundedSender,
};

/// Rebuild in a bounded blocking task. Pull only the chunk currently requested by
/// the strict replay reader; neither a 512-MiB buffer nor a private spool file.
pub(super) async fn restore<S: AsyncRead + AsyncWrite + Unpin>(
    connection: &mut Connection<'_, S>,
    setup: Setup,
    mut progress: Progress,
) -> Result<(Table, Progress), Failure> {
    let deadline = Instant::now() + Duration::from_millis(u64::from(setup.resources.millis));
    let (requests, mut receive) = tokio::sync::mpsc::unbounded_channel();
    let (response, data) = sync_channel(1);
    let reader = Reader {
        requests,
        data,
        deadline,
        chunks: setup.prefix.chunks,
        next: 0,
        current: Cursor::new(Vec::new()),
    };
    let mut worker = tokio::task::spawn_blocking(move || {
        let result = progress.work(|| {
            let resources = setup.resources;
            Table::resume(
                setup.game.to_string(),
                &setup.package,
                &setup.catalog,
                setup.names,
                Recovery {
                    binding: Binding {
                        package: setup.package.digest(&setup.catalog)?,
                        fence: setup.fence,
                        prefix: setup.prefix,
                    },
                    byte_limit: setup.byte_limit,
                    source: reader,
                    resources: ReplayBudget {
                        bytes: resources.bytes as usize,
                        lines: resources.lines as usize,
                        line_bytes: resources.line_bytes as usize,
                        events: resources.events as usize,
                    },
                    deadline,
                },
            )
        });
        result.map(|result| (result, progress))
    });
    let result: Result<_, Failure> = async {
        loop {
            tokio::select! {
                biased;
                result = &mut worker => { let (table, progress) = result??; return Ok((table?, progress)); }
                request = receive.recv() => {
                    let Some(index) = request else { let (table, progress) = (&mut worker).await??; return Ok((table?, progress)); };
                    connection.send(&Control::Read { index }, Vec::new()).await?;
                    let (control, bytes) = tokio::time::timeout_at(deadline.into(), connection.control()).await??;
                    if !matches!(control, Control::Recovery { index: actual } if actual == index) { return Err("Tower recovery order refused".into()); }
                    response.try_send(bytes).map_err(|_| "Tower recovery reader closed")?;
                }
            }
        }
    }.await;
    drop(response);
    drop(receive);
    // A failed transport closes the reader before joining it; no detached replay.
    if result.is_err() && !worker.is_finished() {
        let _ = worker.await;
    }
    result
}

struct Reader {
    requests: UnboundedSender<u64>,
    data: Receiver<Vec<u8>>,
    deadline: Instant,
    chunks: u64,
    next: u64,
    current: Cursor<Vec<u8>>,
}

impl Read for Reader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        if self.current.position() == self.current.get_ref().len() as u64 {
            if self.next == self.chunks {
                return Ok(0);
            }
            self.requests
                .send(self.next)
                .map_err(|_| io::ErrorKind::BrokenPipe)?;
            let chunk = self
                .data
                .recv_timeout(self.deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| io::ErrorKind::TimedOut)?;
            let mut framed = Vec::with_capacity(chunk.len() + 4);
            framed.extend_from_slice(
                &u32::try_from(chunk.len())
                    .map_err(|_| io::ErrorKind::InvalidData)?
                    .to_be_bytes(),
            );
            framed.extend_from_slice(&chunk);
            self.current = Cursor::new(framed);
            self.next += 1;
        }
        self.current.read(output)
    }
}
