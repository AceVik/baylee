//! Compare immutable references after refreshing the complete account document.
use super::{Account, AccountData, Error, LibraryRef, Staged};

pub(super) trait Document {
    fn reload(&mut self) -> Result<Option<LibraryRef>, Error>;
    fn save(&mut self, next: &LibraryRef) -> Result<(), Error>;
}
impl Document for Account {
    fn reload(&mut self) -> Result<Option<LibraryRef>, Error> {
        self.refresh_library()
    }
    fn save(&mut self, next: &LibraryRef) -> Result<(), Error> {
        let mut current = AccountData::read(self.session.document())?;
        current.deck_library = Some(next.clone());
        self.session.save(&self.http, &current.write()?)?;
        self.data = current;
        Ok(())
    }
}
pub(super) fn publish(
    store: &mut impl Document,
    base: Option<&LibraryRef>,
    next: &LibraryRef,
    stage: &mut Staged,
) -> Result<(), Error> {
    for _ in 0..3 {
        let seen = store.reload()?;
        if seen.as_ref() == Some(next) {
            return Ok(());
        }
        if seen.as_ref() != base {
            return Err(Error::Conflict);
        }
        match store.save(next) {
            Ok(()) => return Ok(()),
            // An explicit failed CAS cannot have committed these objects.
            Err(Error::Conflict) => (),
            Err(error @ (Error::Unavailable | Error::Rejected)) => {
                stage.publication_attempted = true;
                match store.reload()? {
                    Some(published) if &published == next => return Ok(()),
                    actual if actual.as_ref() == base => {
                        if error == Error::Rejected {
                            return Err(error);
                        }
                    }
                    _ => return Err(Error::Conflict),
                }
            }
            Err(error) => return Err(error),
        }
    }
    Err(Error::Conflict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decks::Blob;
    use std::collections::VecDeque;

    fn reference(generation: u64, digest: u8) -> LibraryRef {
        LibraryRef {
            generation,
            blob: Blob {
                bytes: 1,
                digest: [digest; 32],
                chunks: Vec::new(),
            },
        }
    }
    enum Reply {
        Committed,
        Lost,
        Conflict,
        Unavailable,
        Revoked,
    }
    struct Pages {
        current: Option<LibraryRef>,
        replies: VecDeque<Reply>,
        writes: usize,
    }
    impl Document for Pages {
        fn reload(&mut self) -> Result<Option<LibraryRef>, Error> {
            Ok(self.current.clone())
        }
        fn save(&mut self, next: &LibraryRef) -> Result<(), Error> {
            self.writes += 1;
            match self.replies.pop_front().expect("unexpected extra write") {
                Reply::Committed => {
                    self.current = Some(next.clone());
                    Ok(())
                }
                Reply::Lost => {
                    self.current = Some(next.clone());
                    Err(Error::Unavailable)
                }
                Reply::Unavailable => Err(Error::Unavailable),
                Reply::Conflict => Err(Error::Conflict),
                Reply::Revoked => Err(Error::Rejected),
            }
        }
    }
    #[test]
    fn lost_publication_is_reconciled_without_a_second_upload_or_write() {
        let mut pages = Pages {
            current: None,
            replies: [Reply::Lost].into(),
            writes: 0,
        };
        let next = reference(1, 1);
        let mut staged = Staged::new();
        publish(&mut pages, None, &next, &mut staged).unwrap();
        publish(&mut pages, None, &next, &mut staged).unwrap();
        assert_eq!(pages.writes, 1);
    }
    #[test]
    fn retryable_failures_reuse_the_original_intent_and_refuse_another_root() {
        let old = reference(1, 1);
        let next = reference(2, 2);
        let mut pages = Pages {
            current: Some(old.clone()),
            replies: [Reply::Conflict, Reply::Unavailable, Reply::Committed].into(),
            writes: 0,
        };
        publish(&mut pages, Some(&old), &next, &mut Staged::new()).unwrap();
        assert_eq!(pages.writes, 3);
        // A racing writer also obtained generation 2, with different objects.
        let fork = reference(2, 3);
        assert!(matches!(
            publish(&mut pages, Some(&old), &fork, &mut Staged::new()),
            Err(Error::Conflict)
        ));
        assert_eq!(pages.writes, 3);
        pages.replies.push_back(Reply::Revoked);
        assert!(
            publish(
                &mut pages,
                Some(&next),
                &reference(3, 1),
                &mut Staged::new()
            )
            .is_err()
        );
        assert!(pages.current.as_ref() == Some(&next));
    }
}
