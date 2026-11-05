//! Require the complete current recipient set before submitting ciphertext.
use baylee_account::Error;
use baylee_protocol::social::{AccountKeys, Sealed};
use std::collections::BTreeSet;

/// No extra, missing or duplicate recipient is silently accepted. The core seals
/// only with this service's independently pinned and confirmed directory.
pub(super) fn validate_wraps<'a>(
    sealed: &Sealed,
    keys: impl Iterator<Item = &'a AccountKeys>,
) -> Result<(), Error> {
    let mut wanted = BTreeSet::new();
    for account in keys {
        if account.devices.is_empty() {
            return Err(Error::Conflict);
        }
        wanted.extend(account.devices.iter().map(|device| device.id.as_str()));
    }
    let present: BTreeSet<_> = sealed.wraps.iter().map(|wrap| wrap.d.as_str()).collect();
    if wanted.is_empty() || present.len() != sealed.wraps.len() || wanted != present {
        return Err(Error::Conflict);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_protocol::social::{DeviceKey, Wrap};
    fn account(id: &str) -> AccountKeys {
        AccountKeys {
            tag: "0007".into(),
            handle: "A#0007".into(),
            devices: vec![DeviceKey {
                id: id.into(),
                ik: id.into(),
                dk: id.into(),
                dsig: id.into(),
                label: "test".into(),
                added_at: 1,
            }],
        }
    }

    #[test]
    fn recipient_sets_are_exact_and_never_modified_on_rejection() {
        let sealed = Sealed {
            hdr: "header".into(),
            n: "nonce".into(),
            ct: "ciphertext".into(),
            epk: "ephemeral".into(),
            sig: "signature".into(),
            wraps: vec![Wrap {
                d: "central".into(),
                k: "wrapped".into(),
            }],
        };
        let current = account("central");
        assert_eq!(validate_wraps(&sealed, [&current].into_iter()), Ok(()));
        let missing = account("missing");
        assert_eq!(
            validate_wraps(&sealed, [&missing].into_iter()),
            Err(Error::Conflict)
        );
        let mut extra = sealed.clone();
        extra.wraps.push(Wrap {
            d: "gateway".into(),
            k: "unwanted".into(),
        });
        assert_eq!(
            validate_wraps(&extra, [&current].into_iter()),
            Err(Error::Conflict)
        );
        let mut duplicate = sealed.clone();
        duplicate.wraps.push(sealed.wraps[0].clone());
        assert_eq!(
            validate_wraps(&duplicate, [&current].into_iter()),
            Err(Error::Conflict)
        );
    }
}
