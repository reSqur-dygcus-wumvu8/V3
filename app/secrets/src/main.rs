//! Application de gestion des secrets OSINT
//! Gère les clés MLA, tokens API et autres secrets sensibles

use std::sync::Arc;
use tokio::sync::Mutex;

// Modules backend
mod backend;
use backend::{
    keyring::KeyringManager,
    ipc::IpcServer,
    crypto::CryptoManager,
};

// Module partagé
use osint_shared::models::AppSettings;

/// Structure globale de l'application Secrets
pub struct AppState {
    pub keyring_manager: Arc<Mutex<KeyringManager>>,
    pub crypto_manager: Arc<Mutex<CryptoManager>>,
    pub ipc_server: Arc<Mutex<Option<IpcServer>>>,
    pub settings: Arc<Mutex<AppSettings>>,
}

fn main() {
    // Initialiser le logger
    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .init();
    
    log::info!("Démarrage de l'application Secrets OSINT...");
    
    // Charger les paramètres
    let settings = Arc::new(Mutex::new(AppSettings::default()));
    let settings_clone = Arc::clone(&settings);
    let settings_inner = settings_clone.blocking_lock();
    let socket_path = settings_inner.secrets_socket_path();
    log::info!("Paramètres chargés");
    
    // Initialiser le gestionnaire de Keychain
    let keyring_manager = KeyringManager::new().expect("Échec de l'initialisation du Keychain");
    log::info!("Gestionnaire Keychain initialisé");
    
    // Initialiser le gestionnaire de chiffrement
    let crypto_manager = CryptoManager::new().expect("Échec de l'initialisation du chiffrement");
    log::info!("Gestionnaire de chiffrement initialisé");
    
    // Créer l'état global
    let app_state = AppState {
        keyring_manager: Arc::new(Mutex::new(keyring_manager)),
        crypto_manager: Arc::new(Mutex::new(crypto_manager)),
        ipc_server: Arc::new(Mutex::new(None)),
        settings,
    };
    
    // Initialiser le serveur IPC
    log::info!("Serveur IPC initialisé sur {}", socket_path.display());
    
    let ipc_server = IpcServer::new(
        &socket_path,
        &app_state.crypto_manager,
    ).expect("Échec de l'initialisation du serveur IPC");
    
    // Mettre à jour l'état avec le serveur IPC
    {
        let mut server_guard = app_state.ipc_server.blocking_lock();
        *server_guard = Some(ipc_server);
    }
    
    // Démarrer le serveur IPC
    log::info!("Démarrage du serveur IPC...");
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    runtime.block_on(async {
        let ipc_server_arc = app_state.ipc_server.clone();
        let guard = ipc_server_arc.lock().await;
        if let Some(ref server) = *guard {
            if let Err(e) = server.start() {
                log::error!("Échec du serveur IPC: {}", e);
                std::process::exit(1);
            }
        }
    });
}


