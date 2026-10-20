//! Module de communication IPC entre les applications

use std::path::PathBuf;
use std::os::unix::net::{UnixListener, UnixStream};
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result};
use serde_json::{json, Value};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};

use osint_shared::ipc::{IpcMessage, IpcMethod, DeriveSessionKeyParams, DeriveSessionKeyResult, StoreSecretParams, GetSecretParams, GetSecretResult};
use crate::backend::crypto::CryptoManager;

type HmacSha256 = Hmac<Sha256>;

/// Serveur IPC pour recevoir les requêtes de l'application principale
pub struct IpcServer {
    pub socket_path: PathBuf,
    pub crypto_manager: Arc<Mutex<CryptoManager>>,
    pub shared_secret: Option<Vec<u8>>,
}

impl IpcServer {
    /// Crée un nouveau serveur IPC
    pub fn new(
        socket_path: &PathBuf,
        crypto_manager: &Arc<Mutex<CryptoManager>>,
    ) -> Result<Self> {
        // Nettoyer le socket existant
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }
        
        Ok(Self {
            socket_path: socket_path.clone(),
            crypto_manager: Arc::clone(crypto_manager),
            shared_secret: None,
        })
    }
    
    /// Définit la clé secrète partagée
    pub fn set_shared_secret(&mut self, secret: Vec<u8>) {
        self.shared_secret = Some(secret);
    }
    
    /// Démarre le serveur IPC
    pub fn start(&self) -> Result<()> {
        let listener = UnixListener::bind(&self.socket_path)?;
        
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut buffer = vec![0; 4096];
                    let n = stream.read(&mut buffer)?;
                    buffer.truncate(n);
                    
                    let message_str = String::from_utf8(buffer)?;
                    let message: IpcMessage = serde_json::from_str(&message_str)?;
                    
                    let response = self.handle_message(&message)?;
                    
                    let response_json = serde_json::to_string(&response)?;
                    stream.write_all(response_json.as_bytes())?;
                }
                Err(e) => {
                    log::error!("Erreur de connexion IPC: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Gère un message IPC
    fn handle_message(&self, message: &IpcMessage) -> Result<IpcMessage> {
        match message.method.clone() {
            IpcMethod::DeriveSessionKey => {
                let params: DeriveSessionKeyParams = serde_json::from_value(message.params.clone())?;
                let result = self.handle_derive_session_key(&params)?;
                Ok(IpcMessage::response(message.id.clone(), json!(result)))
            }
            IpcMethod::StoreSecret => {
                let params: StoreSecretParams = serde_json::from_value(message.params.clone())?;
                self.handle_store_secret(&params)?;
                Ok(IpcMessage::response(message.id.clone(), json!({})))
            }
            IpcMethod::GetSecret => {
                let params: GetSecretParams = serde_json::from_value(message.params.clone())?;
                let result = self.handle_get_secret(&params)?;
                Ok(IpcMessage::response(message.id.clone(), json!(result)))
            }
            _ => {
                Ok(IpcMessage::error(message.id.clone(), "Méthode non implémentée".to_string()))
            }
        }
    }
    
    /// Gère la dérivation de la clé de session
    fn handle_derive_session_key(&self, params: &DeriveSessionKeyParams) -> Result<DeriveSessionKeyResult> {
        let mut runtime = tokio::runtime::Runtime::new()?;
        let mut crypto_manager = runtime.block_on(async {
            self.crypto_manager.lock().await
        });
        
        // Récupérer la clé maître depuis le Keychain
        // (En pratique, cela devrait être fait via le KeyringManager)
        // Pour l'instant, on utilise une clé de test
        let master_key = vec![0u8; 32]; // À remplacer par la vraie clé
        crypto_manager.set_master_key(master_key)?;
        
        let session_key = crypto_manager.derive_session_key(params.context.as_bytes())?;
        
        Ok(DeriveSessionKeyResult {
            session_key: general_purpose::STANDARD.encode(&session_key),
        })
    }
    
    /// Gère le stockage d'un secret
    fn handle_store_secret(&self, _params: &StoreSecretParams) -> Result<()> {
        // À implémenter avec le KeyringManager
        Ok(())
    }
    
    /// Gère la récupération d'un secret
    fn handle_get_secret(&self, _params: &GetSecretParams) -> Result<GetSecretResult> {
        // À implémenter avec le KeyringManager
        Ok(GetSecretResult { value: None })
    }
}

impl Clone for IpcServer {
    fn clone(&self) -> Self {
        Self {
            socket_path: self.socket_path.clone(),
            crypto_manager: Arc::clone(&self.crypto_manager),
            shared_secret: self.shared_secret.clone(),
        }
    }
}

/// Client IPC pour envoyer des requêtes à l'application Secrets
pub struct IpcClient {
    socket_path: PathBuf,
    shared_secret: Option<Vec<u8>>,
}

impl IpcClient {
    /// Crée un nouveau client IPC
    pub fn new(socket_path: PathBuf, shared_secret: Option<Vec<u8>>) -> Self {
        Self {
            socket_path,
            shared_secret,
        }
    }
    
    /// Envoie une requête de dérivation de clé de session
    pub fn derive_session_key(&self, context: &str) -> Result<String> {
        let message = IpcMessage::new(
            IpcMethod::DeriveSessionKey,
            json!(DeriveSessionKeyParams {
                context: context.to_string(),
            }),
        );
        
        let response = self.send_message(message)?;
        
        if response.message_type == osint_shared::ipc::MessageType::Error {
            return Err(anyhow::anyhow!(
                response.error.unwrap_or_default()
            ));
        }
        
        let result: DeriveSessionKeyResult = serde_json::from_value(
            response.result.unwrap_or(Value::Null)
        )?;
        
        Ok(result.session_key)
    }
    
    /// Envoie un message et reçoit la réponse
    fn send_message(&self, mut message: IpcMessage) -> Result<IpcMessage> {
        // Signer le message si une clé secrète est définie
        if let Some(secret) = &self.shared_secret {
            message.signature = message.generate_signature(secret);
        }
        
        let mut stream = UnixStream::connect(&self.socket_path)?;
        
        let message_json = serde_json::to_string(&message)?;
        stream.write_all(message_json.as_bytes())?;
        
        let mut buffer = vec![0; 4096];
        let n = stream.read(&mut buffer)?;
        buffer.truncate(n);
        
        let response_str = String::from_utf8(buffer)?;
        let response: IpcMessage = serde_json::from_str(&response_str)?;
        
        // Vérifier la signature si une clé secrète est définie
        if let Some(secret) = &self.shared_secret {
            if response.verify_signature(secret).is_err() {
                return Err(anyhow::anyhow!("Signature invalide dans la réponse IPC"));
            }
        }
        
        Ok(response)
    }
}

/// Commandes pour IPC (mode CLI)

pub fn get_ipc_status(state: &crate::AppState) -> Result<bool, String> {
    let mut runtime = tokio::runtime::Runtime::new().or_else(|_| Err("Failed to create runtime".to_string()))?;
    let ipc_server = runtime.block_on(async {
        state.ipc_server.lock().await
    });
    Ok(ipc_server.is_some())
}

pub fn get_shared_secret(state: &crate::AppState) -> Result<Option<String>, String> {
    let mut runtime = tokio::runtime::Runtime::new().or_else(|_| Err("Failed to create runtime".to_string()))?;
    let ipc_server = runtime.block_on(async {
        state.ipc_server.lock().await
    });
    if let Some(server) = &*ipc_server {
        if let Some(secret) = &server.shared_secret {
            return Ok(Some(general_purpose::STANDARD.encode(secret)));
        }
    }
    Ok(None)
}

pub fn regenerate_shared_secret(state: &crate::AppState) -> Result<String, String> {
    let mut runtime = tokio::runtime::Runtime::new().or_else(|_| Err("Failed to create runtime".to_string()))?;
    let mut crypto_manager = runtime.block_on(async {
        state.crypto_manager.lock().await
    });
    let mut ipc_server = runtime.block_on(async {
        state.ipc_server.lock().await
    });
    
    let new_secret = crypto_manager.generate_shared_secret();
    
    if let Some(server) = &mut *ipc_server {
        server.set_shared_secret(new_secret.clone());
    }
    
    Ok(general_purpose::STANDARD.encode(&new_secret))
}
