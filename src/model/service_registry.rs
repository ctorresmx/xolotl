use crate::model::service_address::ServiceAddress;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    pub name: String,
    pub environment: String,
    pub address: ServiceAddress,
    pub tags: Vec<String>,
}

pub trait ServiceRegistry {
    fn register(&mut self, entry: ServiceEntry) -> Result<(), RegistryError>;
    fn resolve(&self, name: &str, environment: &str) -> Option<ServiceEntry>;
    fn deregister(&mut self, name: &str, environment: Option<&str>) -> Result<(), RegistryError>;
    fn list(&self) -> Vec<ServiceEntry>;
}

#[derive(Debug)]
pub enum RegistryError {
    AlreadyExists,
    NotFound,
    InternalError(String),
}
