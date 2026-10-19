use crate::AppState;
// Gestion des paramètres de l'application

use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use osint_shared::models::AppSettings as SharedAppSettings;

/// Paramètres spécifiques à l'application principale
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    #[serde(flatten)]
    pub shared: SharedAppSettings,
    pub main: MainSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MainSettings {
    pub last_db_backup: Option<String>,
    pub auto_backup_enabled: bool,
}

impl AppSettings {
    /// Charge les paramètres depuis le fichier de configuration
    pub fn load() -> Result<Self, String> {
        let settings_dir = dirs::config_dir()
            .ok_or("Impossible de trouver le dossier de configuration").map_err(|e| e.to_string())?
            .join("osint_app");
        
        fs::create_dir_all(&settings_dir).map_err(|e| e.to_string())?;
        
        let settings_path = settings_dir.join("settings.json");
        
        if !settings_path.exists() {
            // Créer les paramètres par défaut
            let default_settings = Self {
                shared: SharedAppSettings::default(),
                main: MainSettings {
                    last_db_backup: None,
                    auto_backup_enabled: true,
                },
            };
            
            fs::write(&settings_path, serde_json::to_string_pretty(&default_settings)?)
                .map_err(|e| e.to_string())?;
            
            return Ok(default_settings);
        }
        
        let settings_content = fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&settings_content).map_err(|e| e.to_string())
    }
    
    /// Sauvegarde les paramètres dans le fichier de configuration
    pub fn save(&self) -> Result<(), String> {
        let settings_dir = dirs::config_dir()
            .ok_or("Impossible de trouver le dossier de configuration").map_err(|e| e.to_string())?
            .join("osint_app");
        
        fs::create_dir_all(&settings_dir).map_err(|e| e.to_string())?;
        
        let settings_path = settings_dir.join("settings.json");
        fs::write(&settings_path, serde_json::to_string_pretty(self)?)
            .map_err(|e| e.to_string())
    }
    
    /// Met à jour les paramètres partagés
    pub fn update_shared(&mut self, shared: SharedAppSettings) {
        self.shared = shared;
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            shared: SharedAppSettings::default(),
            main: MainSettings {
                last_db_backup: None,
                auto_backup_enabled: true,
            },
        }
    }
}

/// Récupère les paramètres de l'application

#[allow(dead_code)]
pub async fn get_settings() -> Result<AppSettings, String> {
    AppSettings::load()
}

/// Sauvegarde les paramètres de l'application

#[allow(dead_code)]
pub async fn save_settings(settings: AppSettings) -> Result<(), String> {
    settings.save()
}

/// Récupère le chemin du socket IPC pour la communication avec l'app Secrets
pub async fn get_secrets_socket_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("osint_app");
    
    config_dir.join("secrets.sock")
}
