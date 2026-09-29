//! One module per format. The registry that lists them is
//! [`crate::format::FORMATS`]; the line machinery the two text formats share
//! is [`text`].

pub(crate) mod baylee;
pub(crate) mod json;
pub(crate) mod moxfield;
pub(crate) mod text;
pub(crate) mod yaml;
