//! Owns the OS child until it is reaped, including UI cancellation and errors.
use super::wire::{Bootstrap, FRAME_LIMIT, INITIAL_LIMIT, Input, Output, Reply};
use baylee_worker::{Budget, Limits};
use std::{
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    sync::{mpsc, oneshot},
};

pub(super) fn spawn(
    command: Command,
    initial: Bootstrap,
    input: mpsc::Receiver<Input>,
    output: mpsc::Sender<Reply>,
    stop: oneshot::Receiver<()>,
    pid: Arc<AtomicU32>,
) -> Result<(), String> {
    std::thread::Builder::new()
        .name("baylee-local-supervisor".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match runtime {
                Ok(runtime) => runtime.block_on(run(
                    command,
                    initial,
                    Target::Game {
                        input,
                        output: &output,
                    },
                    stop,
                    &pid,
                    Limits::default(),
                )),
                Err(_) => Err("the local supervisor could not start".into()),
            };
            if let Err(reason) = result {
                bevy::log::warn!("local engine interrupted: {reason}");
                // The child is reaped. If a previous frame fills the queue,
                // dropping this last sender makes the renderer observe the same
                // interruption after that frame, without parking another thread.
                let _ = output.try_send(Reply {
                    messages: vec![super::super::HostMessage::Interrupted],
                    record: Vec::new(),
                    resume: None,
                    done: true,
                });
            }
        })
        .map_err(|_| "the local supervisor thread could not start")?;
    Ok(())
}

pub(super) enum Target<'a> {
    Game {
        input: mpsc::Receiver<Input>,
        output: &'a mpsc::Sender<Reply>,
    },
    Replay {
        progress: &'a AtomicU8,
        bytes: &'a mut Vec<u8>,
    },
}

pub(super) async fn run(
    command: Command,
    initial: Bootstrap,
    target: Target<'_>,
    mut stop: oneshot::Receiver<()>,
    pid: &AtomicU32,
    limits: Limits,
) -> Result<(), String> {
    if !matches!(stop.try_recv(), Err(oneshot::error::TryRecvError::Empty)) {
        return Ok(());
    }
    let mut command = tokio::process::Command::from(command);
    command
        .arg("--local-engine")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env_remove("BAYLEE_SEAT_TOKEN")
        .env_remove("BAYLEE_AGENT_TOKEN")
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|_| "the local engine could not start")?;
    let child_pid = child
        .id()
        .ok_or("the local child has no process identity")?;
    pid.store(child_pid, Ordering::Release);
    // Hold stdin outside Child: wait() otherwise closes it before waiting,
    // which would falsely tell the liveness reader its owner has disappeared.
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let born = Instant::now();
    let budget = Mutex::new(Budget::new(limits, Duration::ZERO));
    let result = tokio::select! {
        biased;
        _ = child.wait() => Err("the local engine exited unexpectedly".into()),
        _ = &mut stop => Ok(()),
        result = exchange(stdin, stdout, initial, target, &budget, born) => result,
        reason = watch(child_pid, &budget, born) => Err(reason),
    };
    // Only this owned Child is signalled, never a sampled or reused PID.
    let _ = child.start_kill();
    child
        .wait()
        .await
        .map_err(|_| "the local engine could not be reaped")?;
    result
}

async fn exchange(
    stdin: Option<tokio::process::ChildStdin>,
    stdout: Option<tokio::process::ChildStdout>,
    initial: Bootstrap,
    target: Target<'_>,
    budget: &Mutex<Budget>,
    born: Instant,
) -> Result<(), String> {
    let (Some(mut stdin), Some(mut stdout)) = (stdin, stdout) else {
        return Err("the local pipes are missing".into());
    };
    let policy = initial.preset()?.house_rules.loop_policy;
    write(
        &mut stdin,
        &baylee_core::wire::encode(&initial),
        INITIAL_LIMIT,
    )
    .await?;
    // Release a resumed tape before constructing and streaming its new copy.
    drop(initial);
    match read(&mut stdout).await? {
        Output::Hello(contract) if contract.accepts(policy) => {}
        _ => return Err("the local engine version differs from its owner".into()),
    }
    match target {
        Target::Game { input, output } => {
            game(&mut stdin, &mut stdout, input, output, budget, born).await
        }
        Target::Replay { progress, bytes } => {
            projection(&mut stdout, progress, bytes, budget, born).await
        }
    }
}

async fn game(
    stdin: &mut tokio::process::ChildStdin,
    stdout: &mut tokio::process::ChildStdout,
    mut input: mpsc::Receiver<Input>,
    output: &mpsc::Sender<Reply>,
    budget: &Mutex<Budget>,
    born: Instant,
) -> Result<(), String> {
    loop {
        let Output::Progress(reply) = read(stdout).await? else {
            return Err("the local engine repeated its handshake".into());
        };
        mark(budget, baylee_worker::IDLE, born)?;
        let done = reply.done;
        output
            .send(reply)
            .await
            .map_err(|_| "the local renderer left")?;
        // Renderer backpressure and human thinking are idle, never work time.
        if done {
            let Some(next) = input.recv().await else {
                return Ok(());
            };
            mark(budget, baylee_worker::BUSY, born)?;
            write(stdin, &baylee_core::wire::encode(&next), FRAME_LIMIT).await?;
        } else {
            mark(budget, baylee_worker::BUSY, born)?;
        }
    }
}

async fn projection(
    stdout: &mut tokio::process::ChildStdout,
    progress: &AtomicU8,
    bytes: &mut Vec<u8>,
    budget: &Mutex<Budget>,
    born: Instant,
) -> Result<(), String> {
    loop {
        let Output::Projection {
            lines,
            percent,
            done,
        } = read(stdout).await?
        else {
            return Err("the local replay response was invalid".into());
        };
        if percent > 100
            || percent < progress.load(Ordering::Relaxed)
            || bytes
                .len()
                .checked_add(lines.len())
                .is_none_or(|size| size > baylee_client_core::bugreport::MAX_INFLATED_BYTES)
        {
            return Err("the local replay exceeded its output contract".into());
        }
        mark(budget, baylee_worker::IDLE, born)?;
        bytes.extend(lines);
        progress.store(percent, Ordering::Relaxed);
        if done {
            return Ok(());
        }
        mark(budget, baylee_worker::BUSY, born)?;
    }
}

fn mark(budget: &Mutex<Budget>, marker: u8, born: Instant) -> Result<(), String> {
    budget
        .lock()
        .map_err(|_| "the local resource monitor failed")?
        .observe(marker, born.elapsed())
        .map_err(|reason| reason.to_string())
}

async fn read(reader: &mut tokio::process::ChildStdout) -> Result<Output, String> {
    let prefix = reader
        .read_u32()
        .await
        .map_err(|_| "the local output pipe closed")?;
    let size = baylee_worker::frame::length(prefix.to_be_bytes(), FRAME_LIMIT)
        .map_err(|_| "the local engine response was too large")?;
    let mut bytes = vec![0; size];
    reader
        .read_exact(&mut bytes)
        .await
        .map_err(|_| "the local output pipe was truncated")?;
    baylee_core::wire::decode(&bytes).map_err(|_| "the local engine response was invalid".into())
}

async fn write(
    writer: &mut tokio::process::ChildStdin,
    bytes: &[u8],
    limit: usize,
) -> Result<(), String> {
    let length = u32::try_from(bytes.len())
        .ok()
        .filter(|n| *n != 0 && bytes.len() <= limit)
        .ok_or("the local engine command was too large")?;
    writer
        .write_all(&length.to_be_bytes())
        .await
        .map_err(|_| "the local input pipe closed")?;
    writer
        .write_all(bytes)
        .await
        .map_err(|_| "the local input pipe closed")?;
    writer
        .flush()
        .await
        .map_err(|_| "the local input pipe closed".into())
}

async fn watch(pid: u32, budget: &Mutex<Budget>, born: Instant) -> String {
    let Ok(mut system) = sysinfo::System::new() else {
        return "the local resource monitor is unavailable".into();
    };
    let pid = sysinfo::Pid::from_u32(pid);
    let mut ticks = tokio::time::interval(Duration::from_millis(250));
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut missing_since = None;
    loop {
        ticks.tick().await;
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[pid]),
            true,
            sysinfo::ProcessRefreshKind::nothing().with_memory(),
        );
        let Ok(budget) = budget.lock() else {
            return "the local resource monitor failed".into();
        };
        let now = born.elapsed();
        let memory = if let Some(process) = system.process(pid) {
            missing_since = None;
            process.memory()
        } else {
            let since = *missing_since.get_or_insert(now);
            if now.saturating_sub(since) >= Duration::from_secs(1) {
                return "the local resource monitor lost its process".into();
            }
            0
        };
        if let Err(reason) = budget.check(now, memory) {
            return reason.to_string();
        }
    }
}

#[cfg(test)]
mod tests;
