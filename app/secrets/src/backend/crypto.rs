//! Module de chiffrement pour les secrets

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce
};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use zeroize::Zeroize;

/// Taille de la clé (256 bits)
const KEY_SIZE: usize = 32;

/// Taille du nonce (96 bits pour AES-GCM)
const NONCE_SIZE: usize = 12;

/// Gestionnaire de chiffrement
pub struct CryptoManager {
    master_key: Option<Vec<u8>>,
}

impl CryptoManager {
    /// Crée un nouveau CryptoManager
    pub fn new() -> Result<Self> {
        Ok(Self {
            master_key: None,
        })
    }
    
    /// Définit la clé maître
    pub fn set_master_key(&mut self, key: Vec<u8>) -> Result<()> {
        if key.len() != KEY_SIZE {
            return Err(anyhow::anyhow!(
                "La clé doit faire {} octets",
                KEY_SIZE
            ));
        }
        self.master_key = Some(key);
        Ok(())
    }
    
    /// Génère une nouvelle clé maître
    pub fn generate_master_key(&mut self) -> Vec<u8> {
        let mut key = vec![0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key);
        self.master_key = Some(key.clone());
        key
    }
    
    /// Chiffre des données avec la clé maître
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let key = self.master_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Aucune clé maître définie"))?;
        
        let cipher = Aes256Gcm::new_from_slice(key)
            .context("Invalid key length")?;
        
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        
        let ciphertext = cipher.encrypt(&nonce, plaintext)
            .context("Encryption failed")?;
        
        // Concaténer nonce + ciphertext
        let mut result = nonce.to_vec();
        result.extend(ciphertext);
        
        Ok(result)
    }
    
    /// Déchiffre des données avec la clé maître
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let key = self.master_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Aucune clé maître définie"))?;
        
        if ciphertext.len() < NONCE_SIZE {
            return Err(anyhow::anyhow!(
                "Ciphertext trop court (attendu au moins {} octets)",
                NONCE_SIZE
            ));
        }
        
        let nonce = Nonce::from_slice(&ciphertext[..NONCE_SIZE]);
        let ciphertext_data = &ciphertext[NONCE_SIZE..];
        
        let cipher = Aes256Gcm::new_from_slice(key)
            .context("Invalid key length")?;
        
        cipher.decrypt(nonce, ciphertext_data)
            .context("Decryption failed")
    }
    
    /// Dérive une clé de session à partir de la clé maître
    pub fn derive_session_key(&self, context: &[u8]) -> Result<Vec<u8>> {
        use hkdf::Hkdf;
        use sha2::Sha256;
        
        let key = self.master_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Aucune clé maître définie"))?;
        
        let hkdf = Hkdf::<Sha256>::new(None, key);
        let mut session_key = vec![0u8; KEY_SIZE];
        hkdf.expand(context, &mut session_key)
            .context("Session key derivation failed")?;
        
        Ok(session_key)
    }
    
    /// Génère une clé secrète partagée pour la communication IPC
    pub fn generate_shared_secret(&mut self) -> Vec<u8> {
        let mut secret = vec![0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut secret);
        secret
    }
    
    /// Efface la clé maître de la mémoire
    pub fn clear_master_key(&mut self) {
        if let Some(mut key) = self.master_key.take() {
            key.zeroize();
        }
    }
}

impl Drop for CryptoManager {
    fn drop(&mut self) {
        self.clear_master_key();
    }
}

/// Clé secrète partagée pour la communication IPC
#[derive(Clone)]
pub struct SharedSecret {
    secret: Arc<Mutex<Vec<u8>>>,
}

impl SharedSecret {
    pub fn new(secret: Vec<u8>) -> Self {
        Self {
            secret: Arc::new(Mutex::new(secret)),
        }
    }
    
    pub async fn get(&self) -> Vec<u8> {
        self.secret.lock().await.clone()
    }
    
    pub async fn set(&self, secret: Vec<u8>) {
        *self.secret.lock().await = secret;
    }
}
