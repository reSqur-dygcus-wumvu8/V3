//! Gestion des secrets via le Keychain macOS

use keyring::{Entry, Error as KeyringError};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};

use osint_shared::models::AppSettings;

/// Gestionnaire du Keychain pour les secrets
pub struct KeyringManager {
    service_name: String,
}

impl KeyringManager {
    /// Crée un nouveau KeyringManager
    pub fn new() -> Result<Self> {
        Ok(Self {
            service_name: "OSINT_Secrets".to_string(),
        })
    }
    
    /// Génère une nouvelle clé maître
    pub fn generate_master_key(&self) -> Result<Vec<u8>> {
        use rand::RngCore;
        use rand::rngs::OsRng;
        
        let mut key = vec![0u8; 32]; // 256 bits
        OsRng.fill_bytes(&mut key);
        
        // Sauvegarder dans le Keychain
        self.save_master_key(&key)?;
        
        Ok(key)
    }
    
    /// Sauvegarde la clé maître dans le Keychain
    pub fn save_master_key(&self, key: &[u8]) -> Result<()> {
        let entry = Entry::new(&self.service_name, "master_key")?;
        entry.set_password(&base64::encode(key))?;
        Ok(())
    }
    
    /// Récupère la clé maître depuis le Keychain
    pub fn get_master_key(&self) -> Result<Vec<u8>> {
        let entry = Entry::new(&self.service_name, "master_key")?;
        let password = entry.get_password()?;
        base64::decode(password).context("Échec du décodage de la clé maître")
    }
    
    /// Supprime la clé maître du Keychain
    pub fn delete_master_key(&self) -> Result<()> {
        let entry = Entry::new(&self.service_name, "master_key")?;
        entry.delete_credential()?;
        Ok(())
    }
    
    /// Vérifie si une clé maître existe
    pub fn has_master_key(&self) -> Result<bool> {
        let entry = Entry::new(&self.service_name, "master_key")?;
        Ok(entry.get_password().is_ok())
    }
    
    /// Sauvegarde un token API
    pub fn save_api_token(&self, service: &str, token: &str) -> Result<()> {
        let entry = Entry::new(&self.service_name, &format!("api_token_{}", service))?;
        entry.set_password(token)?;
        Ok(())
    }
    
    /// Récupère un token API
    pub fn get_api_token(&self, service: &str) -> Result<Option<String>> {
        let entry = Entry::new(&self.service_name, &format!("api_token_{}", service))?;
        match entry.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    /// Supprime un token API
    pub fn delete_api_token(&self, service: &str) -> Result<()> {
        let entry = Entry::new(&self.service_name, &format!("api_token_{}", service))?;
        entry.delete_credential()?;
        Ok(())
    }
    
    /// Liste tous les tokens API sauvegardés
    pub fn list_api_tokens(&self) -> Result<Vec<String>> {
        // Le Keychain ne permet pas de lister toutes les entrées directement.
        // On doit essayer avec des noms connus.
        let known_services = ["mistral", "google", "qwant", "yandex", "twitter", "telegram"];
        let mut tokens = Vec::new();
        
        for service in known_services.iter() {
            if self.get_api_token(service).unwrap_or(None).is_some() {
                tokens.push(service.to_string());
            }
        }
        
        Ok(tokens)
    }
    
    /// Importe une clé maître depuis une chaîne base64
    pub fn import_master_key(&self, base64_key: &str) -> Result<()> {
        let key = base64::decode(base64_key)?;
        self.save_master_key(&key)
    }
    
    /// Exporte la clé maître en base64
    pub fn export_master_key(&self) -> Result<String> {
        let key = self.get_master_key()?;
        Ok(base64::encode(key))
    }
}

/// Commandes Tauri pour la gestion des clés

#[tauri::command]
pub fn generate_master_key(state: tauri::State<crate::AppState>) -> Result<String, String> {
    let keyring_manager = state.keyring_manager.lock().unwrap();
    let key = keyring_manager.generate_master_key()
        .map_err(|e| e.to_string())?;
    Ok(base64::encode(key))
}

#[tauri::command]
pub fn get_master_key_status(state: tauri::State<crate::AppState>) -> Result<bool, String> {
    let keyring_manager = state.keyring_manager.lock().unwrap();
    keyring_manager.has_master_key().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_master_key(
    state: tauri::State<crate::AppState>,
    base64_key: String,
) -> Result<(), String> {
    let keyring_manager = state.keyring_manager.lock().unwrap();
    keyring_manager.import_master_key(&base64_key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_api_token(
    state: tauri::State<crate::AppState>,
    service: String,
    token: String,
) -> Result<(), String> {
    let keyring_manager = state.keyring_manager.lock().unwrap();
    keyring_manager.save_api_token(&service, &token).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_api_token(
    state: tauri::State<crate::AppState>,
    service: String,
) -> Result<Option<String>, String> {
    let keyring_manager = state.keyring_manager.lock().unwrap();
    keyring_manager.get_api_token(&service).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_api_token(
    state: tauri::State<crate::AppState>,
    service: String,
) -> Result<(), String> {
    let keyring_manager = state.keyring_manager.lock().unwrap();
    keyring_manager.delete_api_token(&service).map_err(|e| e.to_string())
}
