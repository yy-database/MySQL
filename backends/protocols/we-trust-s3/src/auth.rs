use crate::error::Result;

pub struct AuthContext {
    pub access_key: String,
    pub secret_key: String,
}

impl AuthContext {
    pub fn new(access_key: String, secret_key: String) -> Self {
        Self {
            access_key,
            secret_key,
        }
    }

    pub fn validate(&self) -> Result<()> {
        // Placeholder for real validation logic
        Ok(())
    }
}
