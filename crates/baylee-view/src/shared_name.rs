//! A name a view shares with whoever interned it.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

/// A public name, shared rather than copied.
///
/// A view names every object of the table, and most of those names already
/// sit in the game's table of names: a `String` here was an allocation and a
/// free per object per view, a third of a searched game's allocations. On the
/// wire it is the string it always was.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct SharedName(Arc<str>);

impl SharedName {
    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for SharedName {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SharedName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SharedName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<Arc<str>> for SharedName {
    fn from(name: Arc<str>) -> Self {
        Self(name)
    }
}

impl From<&str> for SharedName {
    fn from(name: &str) -> Self {
        Self(Arc::from(name))
    }
}

impl From<String> for SharedName {
    fn from(name: String) -> Self {
        Self(Arc::from(name))
    }
}

impl From<&SharedName> for String {
    fn from(name: &SharedName) -> Self {
        name.as_str().to_owned()
    }
}

impl From<SharedName> for String {
    fn from(name: SharedName) -> Self {
        name.as_str().to_owned()
    }
}

impl PartialEq<str> for SharedName {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SharedName {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for SharedName {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<SharedName> for &str {
    fn eq(&self, other: &SharedName) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<SharedName> for String {
    fn eq(&self, other: &SharedName) -> bool {
        self == other.as_str()
    }
}

impl Serialize for SharedName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SharedName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::SharedName;

    #[test]
    fn it_is_a_plain_string_on_the_wire() {
        let name = SharedName::from("Island");
        let json = serde_json::to_string(&name).expect("a name serializes");
        assert_eq!(json, serde_json::to_string("Island").expect("a string"));
        let back: SharedName = serde_json::from_str(&json).expect("a name parses");
        assert_eq!(back, "Island");
    }
}
