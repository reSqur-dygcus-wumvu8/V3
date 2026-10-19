// Application principale OSINT - Mode backend sans UI
// Gère la collecte, capitalisation et exploitation d'informations

use std::sync::Arc;
use tokio::sync::Mutex;
use log::{info, error};

// Modules backend
mod backend;

// Module partagé  
use osint_shared::models::AppSettings;

/// Structure globale de l'application
pub struct AppState {
    pub db: Arc<Mutex<backend::db::Database>>,
    pub tor_manager: Arc<Mutex<backend::tor::TorManager>>,
    pub mla_context: Arc<Mutex<backend::mla::MlaContext>>,
    pub settings: Arc<Mutex<AppSettings>>,
    pub entity_manager: Arc<Mutex<backend::entities::EntityManager>>,
}

#[tokio::main]
async fn main() {
    // Initialiser le logger
    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .init();
    
    info!("Démarrage de l'application OSINT (mode backend)...");
    info!("Application prête pour les tests du backend");
    
    // Garder l'application en vie
    tokio::signal::ctrl_c().await.expect("Échec de l'attente du signal");
    info!("Arrêt de l'application");
}
