use super::{Failure, Players, Request, Response, Uuid, project, response};
use baylee_tower::v2::package::Seeds;
use baylee_tower_store::{Entropy, Reserve, Start};

impl Players {
    pub(super) async fn dispatch(
        &self,
        account: Uuid,
        request: Request,
    ) -> Result<Response, Failure> {
        let page = u8::try_from(baylee_tower_api::PAGE_SIZE).map_err(|_| Failure::Unavailable)?;
        Ok(match request {
            Request::Cohorts { before } => Response::Cohorts {
                consent: baylee_tower_store::CONSENT_REVISION,
                cohorts: self
                    .store
                    .cohorts(before, page)
                    .await?
                    .into_iter()
                    .map(|(id, saved)| publication(id, saved))
                    .collect(),
            },
            Request::Cohort { cohort } => Response::Cohort {
                cohort: Box::new(publication(cohort, self.store.published(cohort).await?)),
            },
            Request::Runs { before } => Response::Runs {
                runs: self
                    .store
                    .runs(account, before, page)
                    .await?
                    .into_iter()
                    .map(project)
                    .collect(),
            },
            Request::Run { run } => response(self.store.get(account, run).await?),
            Request::Locate { run } => Response::Location {
                location: self.store.locate(account, run).await?,
            },
            Request::Hosts { run, before } => Response::Hosts {
                run,
                hosts: self.store.hosts(account, run, before, page).await?,
            },
            Request::Start {
                request,
                cohort,
                starter,
                consent,
            } => response(
                self.store
                    .start(
                        account,
                        request,
                        Start {
                            cohort,
                            starter,
                            consent,
                        },
                        seed()?,
                    )
                    .await?,
            ),
            Request::Command {
                request,
                run,
                revision,
                command,
            } => response(
                self.store
                    .command(account, request, run, revision, &command)
                    .await?,
            ),
            Request::Reserve {
                request,
                run,
                revision,
                host,
            } => response(
                self.store
                    .reserve(
                        account,
                        request,
                        Reserve {
                            run,
                            revision,
                            host,
                        },
                        Entropy {
                            opponent: seed()?,
                            game: Seeds {
                                shuffle: u64::from_le_bytes(seed()?),
                                policy: u64::from_le_bytes(seed()?),
                            },
                        },
                        self.record_bytes,
                    )
                    .await?,
            ),
            Request::WithdrawEchoes {} => {
                self.store.withdraw_echoes(account).await?;
                Response::Withdrawn {}
            }
        })
    }
}

fn seed<const N: usize>() -> Result<[u8; N], Failure> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|_| Failure::Unavailable)?;
    Ok(bytes)
}

fn publication(id: Uuid, saved: baylee_tower_store::Publication) -> baylee_tower_api::Cohort {
    baylee_tower_api::Cohort {
        id,
        contract: saved.cohort,
        catalog: saved.catalog,
        starters: saved.starters,
    }
}
