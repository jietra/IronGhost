// src/ environments/environment_manager.rs

use std::sync::Arc;

use crate::models::EnvironmentKind;
use crate::environments::{
    environment::Environment,
    env_local::LocalEnvironment,
    env_kali::KaliEnvironment,
};

#[derive(Debug, Clone)]
pub struct EnvironmentManager;

impl EnvironmentManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn acquire(
        &self,
        kind: EnvironmentKind,
    ) -> anyhow::Result<Arc<dyn Environment>> {

        match kind {

            EnvironmentKind::Local => 
                Ok(Arc::new(
                    LocalEnvironment::new()
                )),

            EnvironmentKind::Kali => 
                Ok(Arc::new(
                    KaliEnvironment::new()
                )),

        }
    }

    pub async fn release(
        &self,
        _env: Arc<dyn Environment>,
    ) -> anyhow::Result<()> {

        Ok(())

    }
}