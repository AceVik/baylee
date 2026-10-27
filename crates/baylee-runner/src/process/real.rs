//! Supervise input, output and the entire child process group under one deadline.

use super::{Process, Spec};
use anyhow::{Context, Result, bail};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
#[cfg(unix)]
use std::process::Command;
use std::process::{Child, ExitStatus, Output, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

type Reader = JoinHandle<io::Result<Vec<u8>>>;

pub(super) struct RealProcess {
    child: Child,
    started: Instant,
    timeout: Duration,
    cleanup: bool,
    stdout: Option<Reader>,
    stderr: Option<Reader>,
    writer: Option<JoinHandle<io::Result<()>>>,
}

impl RealProcess {
    fn io_finished(&self) -> bool {
        self.stdout.as_ref().is_none_or(JoinHandle::is_finished)
            && self.stderr.as_ref().is_none_or(JoinHandle::is_finished)
            && self.writer.as_ref().is_none_or(JoinHandle::is_finished)
    }

    fn finish_input(&mut self) -> Result<()> {
        if let Some(writer) = self.writer.take() {
            writer
                .join()
                .map_err(|_| anyhow::anyhow!("stdin writer panicked"))?
                .context("writing child stdin")?;
        }
        Ok(())
    }

    fn kill_group(&mut self) {
        // All our children start their own group. Kill descendants as well:
        // killing just the parent leaves inherited pipes open indefinitely.
        #[cfg(unix)]
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{}", self.child.id())])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = self.child.kill();
    }
}

impl Process for RealProcess {
    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        if !self.cleanup && crate::shutdown::requested() {
            self.kill_group();
            bail!("runner shutdown requested");
        }
        let status = self.child.try_wait().context("checking child status")?;
        if status.is_some() && self.io_finished() {
            self.finish_input()?;
            return Ok(status);
        }
        if self.started.elapsed() >= self.timeout {
            self.kill_group();
            bail!("child timed out after {}ms", self.timeout.as_millis());
        }
        Ok(None)
    }

    fn wait(&mut self, timeout: Duration) -> Result<ExitStatus> {
        let started = Instant::now();
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            if started.elapsed() >= timeout {
                self.kill_group();
                bail!("child timed out after {}ms", timeout.as_millis());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn terminate(&mut self) -> Result<()> {
        #[cfg(unix)]
        {
            let _ = Command::new("/bin/kill")
                .args(["-TERM", "--", &format!("-{}", self.child.id())])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            Ok(())
        }
        #[cfg(not(unix))]
        self.kill()
    }

    fn kill(&mut self) -> Result<()> {
        self.kill_group();
        Ok(())
    }
}

impl Drop for RealProcess {
    fn drop(&mut self) {
        self.kill_group();
        let _ = self.child.wait();
        let _ = self.finish_input();
        let _ = read_output(self.stdout.take());
        let _ = read_output(self.stderr.take());
    }
}

fn reader<R: Read + Send + 'static>(reader: R, prefix: Option<String>) -> Reader {
    thread::spawn(move || {
        if let Some(prefix) = prefix {
            for line in BufReader::new(reader).lines() {
                println!("{prefix} {}", line?);
            }
            Ok(Vec::new())
        } else {
            let mut bytes = Vec::new();
            BufReader::new(reader).read_to_end(&mut bytes)?;
            Ok(bytes)
        }
    })
}

fn read_output(reader: Option<Reader>) -> Result<Vec<u8>> {
    reader.map_or_else(
        || Ok(Vec::new()),
        |reader| {
            reader
                .join()
                .map_err(|_| anyhow::anyhow!("output reader panicked"))?
                .context("reading child output")
        },
    )
}

fn start(spec: &Spec, capture: bool) -> Result<RealProcess> {
    if !spec.cleanup && crate::shutdown::requested() {
        bail!("runner shutdown requested");
    }
    let mut command = spec.command();
    command.stdin(if spec.stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    if capture {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    } else if let Some(path) = &spec.log {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let out = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        let err = out.try_clone().context("cloning the log file")?;
        command.stdout(Stdio::from(out)).stderr(Stdio::from(err));
    } else if spec.prefix.is_some() {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    } else {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }
    // Own the child before doing anything fallible with its pipes.
    let mut process = RealProcess {
        child: command
            .spawn()
            .with_context(|| format!("spawning {}", spec.program))?,
        started: Instant::now(),
        timeout: spec.timeout,
        cleanup: spec.cleanup,
        stdout: None,
        stderr: None,
        writer: None,
    };
    let prefix = if capture { None } else { spec.prefix.clone() };
    process.stdout = process
        .child
        .stdout
        .take()
        .map(|out| reader(out, prefix.clone()));
    process.stderr = process.child.stderr.take().map(|err| reader(err, prefix));
    if let Some(bytes) = &spec.stdin {
        let mut input = process.child.stdin.take().context("opening child stdin")?;
        let bytes = bytes.clone();
        process.writer = Some(thread::spawn(move || input.write_all(&bytes)));
    }
    Ok(process)
}

pub(super) fn spawn(spec: &Spec) -> Result<RealProcess> {
    start(spec, false)
}

pub(super) fn output(spec: &Spec) -> Result<Output> {
    let mut process = start(spec, true)?;
    let status = process.wait(spec.timeout)?;
    Ok(Output {
        status,
        stdout: read_output(process.stdout.take())?,
        stderr: read_output(process.stderr.take())?,
    })
}
