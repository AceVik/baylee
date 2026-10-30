//! `lane login`, refused since the cutover of 30.10.2026. It typed a
//! username and password into the client's gateway sign-in, and no gateway
//! signs anybody in with a password any more (`docs/central-login.md`
//! §"Cutover"). Its rewrite to Baylee Central's sign-in is #406.
use anyhow::{Result, bail};

/// Why `lane login` does nothing, and the ways in that are left.
pub(super) const REFUSED: &str = "lane login is refused: a gateway signs nobody in with a password \
     any more (docs/central-login.md, \"Cutover\"). Sign the lane's client in through Baylee \
     Central's sign-in, or come in as a guest on a gateway that takes guests; the rewrite to \
     Central's sign-in is https://github.com/AceVik/baylee/issues/406";

/// Refuses before anything is read or pressed: none of the old command's
/// arguments, a password among them, is looked at.
pub(super) fn login() -> Result<()> {
    bail!(REFUSED)
}
