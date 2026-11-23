//! Real installed agent and per-match engine processes, each with a private gateway store.
use super::{Registered, common};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;
use uuid::Uuid;

pub(super) struct Host {
    agent: Agent,
    pub(super) registered: Registered,
}

impl Host {
    pub(super) fn start(rt: &Runtime, db: &DatabaseConnection, registered: Registered) -> Self {
        rt.block_on(common::ranked::seed(&registered.gateway));
        let gateway_bin = Path::new(env!("CARGO_BIN_EXE_baylee-gateway"));
        let sibling =
            |name| gateway_bin.with_file_name(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        let mut cmd = Command::new(sibling("baylee-agent"));
        cmd.env_clear()
            .env("BAYLEE_GATEWAY", &registered.origin)
            .env("BAYLEE_AGENT_TOKEN", &registered.gateway.agent_token)
            .env("BAYLEE_AGENT_NAME", "replication-e2e-agent")
            .env("BAYLEE_AGENT_CAPACITY", "1")
            .env("BAYLEE_ENGINE_BIN", sibling("baylee-engine-server"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
        let mut agent = Agent(cmd.spawn().expect("start the compiled production agent"));
        let id = Uuid::from_bytes(registered.id);
        let until = Instant::now() + Duration::from_secs(65);
        loop {
            assert!(
                agent.0.try_wait().unwrap().is_none(),
                "agent terminated before advertising"
            );
            let query = Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT EXISTS (SELECT 1 FROM central_identity.ranked_hosts WHERE host_id=$1
                 AND capacity>0 AND octet_length(compatibility)=32) AS ready",
                [id.into()],
            );
            let row = rt.block_on(db.query_one_raw(query)).unwrap().unwrap();
            if row.try_get::<bool>("", "ready").unwrap() {
                break;
            }
            assert!(
                Instant::now() < until,
                "verified agent never advertised a clean compatible build"
            );
            std::thread::sleep(Duration::from_millis(250));
        }
        Self { agent, registered }
    }

    pub(super) fn crash_gateway(&mut self) {
        assert!(self.agent.0.try_wait().unwrap().is_none());
        self.registered.gateway.crash();
    }
}

struct Agent(Child);

impl Drop for Agent {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(Some(_))) {
            return;
        }
        #[cfg(unix)]
        {
            let pid = self.0.id().to_string();
            let _ = Command::new("/bin/kill").args(["-TERM", &pid]).status();
            let until = Instant::now() + Duration::from_secs(5);
            while Instant::now() < until {
                if matches!(self.0.try_wait(), Ok(Some(_))) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            // The group leader has not yet been reaped. It and all engines in
            // its private process group belong exclusively to this test.
            let _ = Command::new("/bin/kill")
                .args(["-KILL", "--", &format!("-{pid}")])
                .status();
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
