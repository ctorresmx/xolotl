#[derive(Debug, Clone)]
pub struct ServiceEntry {
    pub name: String,
    pub environment: String,
    pub address: String,
    pub tags: Vec<String>,
}

pub trait ServiceRegistry {
    fn register(&mut self, entry: ServiceEntry) -> Result<(), RegistryError>;
    fn resolve(&self, name: &str, environment: &str) -> Option<ServiceEntry>;
    fn unregister(&mut self, name: &str, environment: &str) -> Result<(), RegistryError>;
    fn list(&self) -> Vec<ServiceEntry>;
}

#[derive(Debug)]
pub enum RegistryError {
    AlreadyExists,
    NotFound,
    InternalError(String),
}
