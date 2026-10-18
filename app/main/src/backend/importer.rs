//! Module d'import de fichiers
//! Gère l'import, la déduplication et le traitement initial

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use sha2::{Sha256, Digest};
use log::{info, error, warn};

use crate::backend::db::Database;
use osint_shared::models::EntityType;

/// Résultat d'un import de fichier
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub file_id: Option<i64>,
    pub message: String,
    pub duplicate: bool,
    pub hash: String,
}

/// Gestionnaire d'import de fichiers
pub struct FileImporter {
    db: Arc<Mutex<Database>>,
}

impl FileImporter {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    /// Importe un fichier depuis son contenu binaire
    pub async fn import_file(
        &self,
        name: String,
        data: Vec<u8>,
        source_rating: String,
        date: String,
        hash: Option<String>,
        watch_id: Option<i64>,
    ) -> Result<ImportResult, String> {
        // Calculer le hash SHA-256 si non fourni
        let file_hash = if let Some(h) = hash {
            h
        } else {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            format!("{:x}", hasher.finalize())
        };

        // Vérifier si le fichier existe déjà (déduplication)
        if self.is_duplicate(&file_hash).await? {
            return Ok(ImportResult {
                success: true,
                file_id: None,
                message: "Fichier déjà importé (doublon détecté)".to_string(),
                duplicate: true,
                hash: file_hash,
            });
        }

        // Sauvegarder le fichier localement (optionnel)
        let file_path = self.save_file_locally(&name, &data).await?;

        // Créer ou obtenir la source
        let source_id = self.get_or_create_source(&name, &source_rating).await?;

        // Sauvegarder dans la base de données
        let file_id = self.save_to_database(
            name,
            file_path.to_string_lossy().to_string(),
            date,
            source_id,
            &file_hash,
            watch_id,
        ).await?;

        // Lancer le traitement NER en arrière-plan
        self.process_with_ner(file_id, &data).await?;

        Ok(ImportResult {
            success: true,
            file_id: Some(file_id),
            message: "Fichier importé avec succès".to_string(),
            duplicate: false,
            hash: file_hash,
        })
    }

    /// Vérifie si un fichier est un doublon
    async fn is_duplicate(&self, hash: &str) -> Result<bool, String> {
        let db = self.db.lock().unwrap();
        
        let count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM imported_files WHERE hash = ?",
            [hash],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        Ok(count > 0)
    }

    /// Sauvegarde le fichier localement
    async fn save_file_locally(&self, name: &str, data: &[u8]) -> Result<PathBuf, String> {
        // Créer le dossier de stockage
        let storage_dir = dirs::data_dir()
            .ok_or("Impossible de trouver le dossier de données")?
            .join("osint_app")
            .join("imports");
        
        std::fs::create_dir_all(&storage_dir).map_err(|e| e.to_string())?;

        // Générer un nom de fichier unique
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let sanitized_name = name.replace(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '_', "_");
        let file_path = storage_dir.join(format!("{}_{}_{}", timestamp, sanitized_name));

        // Sauvegarder le fichier
        std::fs::write(&file_path, data).map_err(|e| e.to_string())?;

        Ok(file_path)
    }

    /// Crée ou obtient une source
    async fn get_or_create_source(&self, name: &str, rating: &str) -> Result<i64, String> {
        let db = self.db.lock().unwrap();

        // Vérifier si la source existe déjà
        let existing_source: Option<i64> = db.conn.query_row(
            "SELECT id FROM sources WHERE name = ?",
            [name],
            |row| row.get(0),
        ).optional().map_err(|e| e.to_string())?;

        if let Some(id) = existing_source {
            // Ajouter la cotation si elle n'existe pas
            let count: i64 = db.conn.query_row(
                "SELECT COUNT(*) FROM source_ratings WHERE source_id = ? AND reliability_code = ?",
                (id, rating),
                |row| row.get(0),
            ).map_err(|e| e.to_string())?;

            if count == 0 {
                db.conn.execute(
                    "INSERT INTO source_ratings (source_id, reliability_code) VALUES (?, ?)",
                    (id, rating),
                ).map_err(|e| e.to_string())?;
            }

            return Ok(id);
        }

        // Créer une nouvelle source
        let tx = db.conn.transaction().map_err(|e| e.to_string())?;
        
        let source_id = tx.execute(
            "INSERT INTO sources (name, created_at) VALUES (?, datetime('now'))",
            [name],
        ).map_err(|e| e.to_string())?;

        // Ajouter la cotation
        tx.execute(
            "INSERT INTO source_ratings (source_id, reliability_code) VALUES (?, ?)",
            (source_id as i64, rating),
        ).map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;

        Ok(source_id as i64)
    }

    /// Sauvegarde le fichier dans la base de données
    async fn save_to_database(
        &self,
        name: String,
        path: String,
        date: String,
        source_id: i64,
        hash: &str,
        watch_id: Option<i64>,
    ) -> Result<i64, String> {
        let db = self.db.lock().unwrap();

        let file_id = db.conn.execute(
            "INSERT INTO imported_files (name, path, import_date, source_id, hash) VALUES (?, ?, ?, ?, ?)",
            (name, path, date, source_id, hash),
        ).map_err(|e| e.to_string())?;

        // Lier au watch si fourni
        if let Some(watch_id) = watch_id {
            // Mettre à jour le fichier avec le watch_id
            db.conn.execute(
                "UPDATE imported_files SET watch_id = ? WHERE id = ?",
                (watch_id, file_id as i64),
            ).map_err(|e| e.to_string())?;
        }

        Ok(file_id as i64)
    }

    /// Lance le traitement NER sur le fichier
    async fn process_with_ner(&self, file_id: i64, data: &[u8]) -> Result<(), String> {
        // Extraire le texte du fichier
        let text = match std::str::from_utf8(data) {
            Ok(t) => t.to_string(),
            Err(_) => {
                warn!("Le fichier {} n'est pas du texte UTF-8, NER ignoré", file_id);
                return Ok(());
            }
        };

        // Pour l'instant, on ne fait rien avec le texte
        // Dans une version future, on appellera l'API Mistral pour le NER
        info!("Traitement NER à implémenter pour le fichier {}", file_id);

        Ok(())
    }

    /// Vérifie si un fichier existe déjà (pour l'API Tauri)
    pub fn check_file_duplicate(&self, hash: String) -> Result<bool, String> {
        self.is_duplicate(&hash)
    }
}

/// Commandes Tauri pour l'import de fichiers


#[tauri::command]
pub async fn import_file(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    name: String,
    data: Vec<u8>,
    source_rating: String,
    date: String,
    hash: Option<String>,
    watch_id: Option<i64>,
) -> Result<ImportResult, String> {
    let importer = FileImporter::new(Arc::clone(&state.db));
    importer.import_file(name, data, source_rating, date, hash, watch_id).await
}


#[tauri::command]
pub fn check_file_duplicate(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    hash: String,
) -> Result<bool, String> {
    let importer = FileImporter::new(Arc::clone(&state.db));
    importer.check_file_duplicate(hash)
}


#[tauri::command]
pub fn get_source_by_name(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    name: String,
) -> Result<Option<i64>, String> {
    let db = state.db.lock().unwrap();
    
    let source_id: Option<i64> = db.conn.query_row(
        "SELECT id FROM sources WHERE name = ?",
        [name],
        |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;

    Ok(source_id)
}


#[tauri::command]
pub fn create_source(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    name: String,
    description: Option<String>,
    reliability: String,
) -> Result<i64, String> {
    let importer = FileImporter::new(Arc::clone(&state.db));
    importer.get_or_create_source(&name, &reliability)
}


#[tauri::command]
pub fn process_file_with_ner(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    file_id: i64,
    watch_id: Option<i64>,
) -> Result<(), String> {
    // Pour l'instant, on ne fait rien
    // Cela sera implémenté dans une version future
    info!("Traitement NER pour le fichier {} (veille {})", file_id, watch_id.unwrap_or(0));
    Ok(())
}
