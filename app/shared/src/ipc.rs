//! Protocole IPC pour la communication entre les applications

use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Version du protocole IPC
pub const IPC_PROTOCOL_VERSION: &str = "1.0";

/// Types de messages IPC
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MessageType {
    Request,
    Response,
    Error,
}

/// Méthodes IPC disponibles
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IpcMethod {
    // Gestion des clés
    GetMasterKey,
    DeriveSessionKey,
    StoreSecret,
    GetSecret,
    DeleteSecret,
    // Gestion des tokens API
    GetApiToken,
    StoreApiToken,
}

/// Message IPC
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IpcMessage {
    pub version: String,
    pub message_type: MessageType,
    pub id: String, // UUID
    pub method: IpcMethod,
    pub params: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub signature: String, // HMAC-SHA256
    pub timestamp: u64, // Unix timestamp
    pub nonce: String, // Pour éviter le replay
}

impl IpcMessage {
    /// Crée un nouveau message IPC
    pub fn new(method: IpcMethod, params: serde_json::Value) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            version: IPC_PROTOCOL_VERSION.to_string(),
            message_type: MessageType::Request,
            id: uuid::Uuid::new_v4().to_string(),
            method,
            params,
            result: None,
            error: None,
            signature: String::new(),
            timestamp,
            nonce: uuid::Uuid::new_v4().to_string(),
        }
    }
    
    /// Crée une réponse
    pub fn response(request_id: String, result: serde_json::Value) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            version: IPC_PROTOCOL_VERSION.to_string(),
            message_type: MessageType::Response,
            id: request_id,
            method: IpcMethod::GetMasterKey, // Non utilisé pour les réponses
            params: serde_json::Value::Null,
            result: Some(result),
            error: None,
            signature: String::new(),
            timestamp,
            nonce: uuid::Uuid::new_v4().to_string(),
        }
    }
    
    /// Crée une erreur
    pub fn error(request_id: String, error: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            version: IPC_PROTOCOL_VERSION.to_string(),
            message_type: MessageType::Error,
            id: request_id,
            method: IpcMethod::GetMasterKey, // Non utilisé pour les erreurs
            params: serde_json::Value::Null,
            result: None,
            error: Some(error),
            signature: String::new(),
            timestamp,
            nonce: uuid::Uuid::new_v4().to_string(),
        }
    }
    
    /// Génère la signature du message
    pub fn generate_signature(&self, secret: &[u8]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        
        type HmacSha256 = Hmac<Sha256>;
        
        let mut mac = HmacSha256::new_from_slice(secret)
            .expect("Invalid HMAC key length");
        
        // Sérialiser le message sans la signature
        let message_without_signature = Self {
            signature: String::new(),
            ..self.clone()
        };
        
        let message_bytes = serde_json::to_vec(&message_without_signature)
            .expect("Failed to serialize message");
        
        mac.update(&message_bytes);
        let result = mac.finalize().into_bytes();
        hex::encode(result)
    }
    
    /// Vérifie la signature du message
    pub fn verify_signature(&self, secret: &[u8]) -> Result<bool, String> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        
        type HmacSha256 = Hmac<Sha256>;
        
        let mut mac = HmacSha256::new_from_slice(secret)
            .map_err(|e| e.to_string())?;
        
        // Sérialiser le message sans la signature
        let message_without_signature = Self {
            signature: String::new(),
            ..self.clone()
        };
        
        let message_bytes = serde_json::to_vec(&message_without_signature)
            .map_err(|e| e.to_string())?;
        
        mac.update(&message_bytes);
        Ok(mac.verify_slice(self.signature.as_bytes())
            .is_ok())
    }
    
    /// Vérifie que le message n'est pas un replay
    pub fn is_valid_timestamp(&self, max_age_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        now - self.timestamp <= max_age_seconds
    }
}

/// Paramètres pour la requête de dérivation de clé de session
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeriveSessionKeyParams {
    pub context: String,
}

/// Résultat de la dérivation de clé de session
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeriveSessionKeyResult {
    pub session_key: String, // Base64 encoded
}

/// Paramètres pour le stockage d'un secret
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoreSecretParams {
    pub key: String,
    pub value: String,
}

/// Paramètres pour la récupération d'un secret
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetSecretParams {
    pub key: String,
}

/// Résultat de la récupération d'un secret
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetSecretResult {
    pub value: Option<String>,
}
