use crate::Error;

pub struct Entry;

impl Entry {
    pub fn new(_: &str, _: &str) -> Result<Self, Error> {
        Err(Error::NoDefaultStore)
    }
    pub fn store_status() -> Result<(), Error> {
        Err(Error::NoDefaultStore)
    }
    pub fn get_credential(&self) -> Result<(), Error> {
        Err(Error::NoDefaultStore)
    }
    pub fn get_password(&self) -> Result<String, Error> {
        Err(Error::NoDefaultStore)
    }
    pub fn set_password(&self, _: &str) -> Result<(), Error> {
        Err(Error::NoDefaultStore)
    }
    pub fn delete_credential(&self) -> Result<(), Error> {
        Err(Error::NoDefaultStore)
    }
}
