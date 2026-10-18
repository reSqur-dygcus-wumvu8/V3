//! Application de gestion des secrets OSINT
//! Gère les clés MLA, tokens API et autres secrets sensibles

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
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
    let settings = AppSettings::load().expect("Échec du chargement des paramètres");
    log::info!("Paramètres chargés");
    
    // Initialiser le gestionnaire de Keychain
    let keyring_manager = KeyringManager::new().expect("Échec de l'initialisation du Keychain");
    log::info!("Gestionnaire Keychain initialisé");
    
    // Initialiser le gestionnaire de chiffrement
    let crypto_manager = CryptoManager::new().expect("Échec de l'initialisation du chiffrement");
    log::info!("Gestionnaire de chiffrement initialisé");
    
    // Initialiser le serveur IPC
    let ipc_server = IpcServer::new(
        &settings.secrets_socket_path(),
        &crypto_manager,
    ).expect("Échec de l'initialisation du serveur IPC");
    log::info!("Serveur IPC initialisé sur {}", settings.secrets_socket_path());
    
    // Démarrer le serveur IPC en arrière-plan
    let ipc_server_clone = ipc_server.clone();
    tokio::spawn(async move {
        if let Err(e) = ipc_server_clone.lock().unwrap().start() {
            log::error!("Échec du serveur IPC: {}", e);
        }
    });
    
    // Créer l'état global
    let app_state = AppState {
        keyring_manager: Arc::new(Mutex::new(keyring_manager)),
        crypto_manager: Arc::new(Mutex::new(crypto_manager)),
        ipc_server: Arc::new(Mutex::new(Some(ipc_server))),
        settings: Arc::new(Mutex::new(settings)),
    };
    
    // Configurer Tauri
    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            // Initialiser la fenêtre
            let window = app.get_webview_window("main").unwrap();
            window.set_title("OSINT Secrets - Gestion des clés")?;
            
            // Définir la taille minimale
            window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize { width: 600.0, height: 400.0 })))?;
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Gestion des clés MLA
            backend::keyring::generate_master_key,
            backend::keyring::get_master_key_status,
            backend::keyring::import_master_key,
            
            // Gestion des tokens API
            backend::keyring::save_api_token,
            backend::keyring::get_api_token,
            backend::keyring::delete_api_token,
            
            // Gestion IPC
            backend::ipc::get_ipc_status,
            backend::ipc::get_shared_secret,
            backend::ipc::regenerate_shared_secret,
        ])
        .run(tauri::generate_context!())
        .expect("Échec de l'exécution de l'application Secrets");
}

// Clone pour IpcServer
impl Clone for IpcServer {
    fn clone(&self) -> Self {
        Self {
            socket_path: self.socket_path.clone(),
            crypto_manager: Arc::clone(&self.crypto_manager),
        }
    }
}
