//! Scope challenge replies to the exact account worker, never the selected gate.
impl super::Handle {
    pub(crate) fn same_session(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.active, &other.active)
    }
}
