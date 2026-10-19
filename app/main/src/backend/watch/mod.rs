use crate::AppState;
// Module de veille internet
// Gère la configuration, l'exécution et le traitement des veilles

pub mod scrapers;
pub mod scheduler;
pub mod processor;

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::backend::{db::Database, tor::TorClient};
use osint_shared::models::EntityType;

/// Configuration d'une veille
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watch {
    pub id: Option<i64>,
    pub name: String,
    pub query: String,
    pub language: String,
    pub sources: Vec<String>,
    pub frequency_minutes: u32,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Résultat d'une veille
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchResult {
    pub id: Option<i64>,
    pub watch_id: i64,
    pub title: String,
    pub url: String,
    pub content: String,
    pub translated_content: Option<String>,
    pub source_name: String,
    pub published_at: Option<String>,
    pub fetched_at: Option<String>,
    pub is_processed: bool,
    pub metadata: Option<serde_json::Value>,
    pub entities: Option<Vec<EntityReference>>,
}

/// Référence à une entité détectée
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityReference {
    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub name: String,
    pub confidence: f64,
    pub matched: bool,
}

/// Résultat traité avec entités et cotation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedWatchResult {
    pub id: i64,
    pub watch_id: i64,
    pub title: String,
    pub url: String,
    pub content: String,
    pub translated_content: Option<String>,
    pub source_name: String,
    pub published_at: Option<String>,
    pub fetched_at: Option<String>,
    pub entities: Vec<EntityReference>,
    pub proposed_rating: String, // Format: "A1", "B2", etc.
    pub source_rating: String, // Fiabilité de la source (A-F)
    pub info_rating: String, // Crédibilité de l'information (1-6)
}

/// Groupe d'événements pour la capitalisation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventGroup {
    pub event: String,
    pub date: Option<String>,
    pub results: Vec<ProcessedWatchResult>,
    pub proposed_rating: String,
}

/// Client pour les appels API externes (Mistral, etc.)
#[derive(Clone)]
pub struct ApiClient {
    pub tor_client: TorClient,
    pub mistral_api_key: Option<String>,
    pub mistral_endpoint: String,
}

impl ApiClient {
    pub async fn new(tor_client: TorClient, mistral_api_key: Option<String>) -> Self {
        Self {
            tor_client,
            mistral_api_key,
            mistral_endpoint: "https://api.mistral.ai/v1".to_string(),
        }
    }

    /// Appelle l'API Mistral avec retry
    pub async fn call_mistral(
        &self,
        prompt: &str,
        max_tokens: u32,
        temperature: f64,
    ) -> Result<String, String> {
        if self.mistral_api_key.is_none() {
            return Err("Clé API Mistral non configurée".to_string());
        }

        let client = self.tor_client.client.clone();
        let api_key = self.mistral_api_key.as_ref().unwrap();
        let endpoint = format!("{}/chat/completions", self.mistral_endpoint);

        let request_body = serde_json::json!({
            "model": "mistral-large",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": max_tokens,
            "temperature": temperature,
        });

        let response = client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Erreur Mistral API: {} - {}", status, body));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Erreur de parsing JSON: {}", e))?;

        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first_choice) = choices.first() {
                if let Some(message) = first_choice.get("message") {
                    if let Some(content) = message.get("content") {
                        return Ok(content.as_str().unwrap_or("").to_string());
                    }
                }
            }
        }

        Err("Réponse Mistral invalide".to_string())
    }

    /// Traduit un texte via Mistral
    pub async fn translate(&self, text: &str, target_language: &str) -> Result<String, String> {
        let prompt = format!(
            "Traduis le texte suivant en {} sans ajouter d'explications ou de commentaires. \
             Texte à traduire: \"{}\"",
            target_language, text
        );

        self.call_mistral(&prompt, 1000, 0.1).await
    }

    /// Extrait les entités d'un texte via Mistral
    pub async fn extract_entities(&self, text: &str) -> Result<Vec<EntityReference>, String> {
        let prompt = format!(
            "Analyse le texte suivant et extrait toutes les entités (personnes, organisations, lieux, événements, objets) \
             au format JSON. Pour chaque entité, retourne : \
             - type: 'individual', 'organization', 'location', 'event', ou 'object' \
             - name: le nom de l'entité \
             - confidence: un score de confiance entre 0 et 1 \
             \
             Texte: \"{}\" \
             \
             Réponds UNIQUEMENT avec un tableau JSON valide, sans autre texte.",
            text
        );

        let response = self.call_mistral(&prompt, 2000, 0.1).await?;

        // Nettoyer la réponse pour extraire le JSON
        let json_str = response.trim();
        let entities: Vec<EntityReference> = serde_json::from_str(json_str)
            .map_err(|e| format!("Erreur de parsing des entités: {}", e))?;

        Ok(entities)
    }

    /// Propose une cotation pour une information
    pub async fn propose_rating(
        &self,
        source_rating: &str,
        text: &str,
        entities: &[EntityReference],
    ) -> Result<String, String> {
        let prompt = format!(
            "En tant qu'analyste en renseignement, propose une cotation OTAN pour l'information suivante. \
             La cotation doit être au format 'Xy' où X est la fiabilité de la source (A-F) et y est la crédibilité de l'information (1-6). \
             \
             Source: Fiabilité {}. \
             Information: \"{}\" \
             Entités détectées: {} \
             \
             Réponds UNIQUEMENT avec la cotation au format 'Xy' (ex: 'A1', 'B2', etc.), sans explication.",
            source_rating,
            text,
            serde_json::to_string(entities).unwrap_or_default()
        );

        let response = self.call_mistral(&prompt, 100, 0.1).await?;
        let cotation = response.trim().to_string();

        // Valider le format de la cotation
        if cotation.len() == 2 {
            let reliability = &cotation[..1];
            let credibility = &cotation[1..];
            
            if ['A', 'B', 'C', 'D', 'E', 'F'].contains(&reliability.chars().next().unwrap()) &&
               ['1', '2', '3', '4', '5', '6'].contains(&credibility.chars().next().unwrap()) {
                return Ok(cotation);
            }
        }

        // Fallback: cotation par défaut
        Ok(format!("{}2", source_rating))
    }
}

/// Gestionnaire des veilles
pub struct WatchManager {
    pub db: Arc<Mutex<Database>>,
    pub tor_client: TorClient,
    pub api_client: ApiClient,
}

impl WatchManager {
    pub async fn new(
        db: Arc<Mutex<Database>>,
        tor_client: TorClient,
        api_client: ApiClient,
    ) -> Self {
        Self {
            db,
            tor_client,
            api_client,
        }
    }

    /// Récupère toutes les veilles
    pub async fn get_watches(&self) -> Result<Vec<Watch>, String> {
        let db = self.db.lock().await;
        let mut stmt = db.conn.prepare(
            "SELECT id, name, query, language, sources, frequency_minutes, is_active, created_at, updated_at 
             FROM watches 
             ORDER BY created_at DESC"
        ).map_err(|e| e.to_string())?;

        let mut watches = Vec::new();
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let watch = Watch {
                id: row.get(0).ok(),
                name: row.get(1).unwrap_or_default(),
                query: row.get(2).unwrap_or_default(),
                language: row.get(3).unwrap_or_default(),
                sources: serde_json::from_str(&row.get::<_, String>(4).unwrap_or_default())
                    .unwrap_or_default(),
                frequency_minutes: row.get(5).unwrap_or(60),
                is_active: row.get(6).unwrap_or(true),
                created_at: row.get(7).ok(),
                updated_at: row.get(8).ok(),
            };
            watches.push(watch);
        }

        Ok(watches)
    }

    /// Ajoute une nouvelle veille
    pub async fn add_watch(&self, watch: Watch) -> Result<i64, String> {
        let db = self.db.lock().await;
        let sources_json = serde_json::to_string(&watch.sources).unwrap_or_default();
        let now = chrono::Utc::now().to_rfc3339();

        let row_id = db.conn.execute(
            "INSERT INTO watches (name, query, language, sources, frequency_minutes, is_active, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (
                watch.name,
                watch.query,
                watch.language,
                sources_json,
                watch.frequency_minutes,
                watch.is_active,
                now.clone(),
                now,
            ),
        ).map_err(|e| e.to_string())?;

        Ok(row_id as i64)
    }

    /// Met à jour une veille
    pub async fn update_watch(&self, watch: Watch) -> Result<(), String> {
        let db = self.db.lock().await;
        let sources_json = serde_json::to_string(&watch.sources).unwrap_or_default();
        let now = chrono::Utc::now().to_rfc3339();

        if let Some(id) = watch.id {
            db.conn.execute(
                "UPDATE watches 
                 SET name = ?, query = ?, language = ?, sources = ?, frequency_minutes = ?, is_active = ?, updated_at = ? 
                 WHERE id = ?",
                (
                    watch.name,
                    watch.query,
                    watch.language,
                    sources_json,
                    watch.frequency_minutes,
                    watch.is_active,
                    now,
                    id,
                ),
            ).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Supprime une veille
    pub async fn delete_watch(&self, id: i64) -> Result<(), String> {
        let db = self.db.lock().await;
        db.conn.execute(
            "DELETE FROM watches WHERE id = ?",
            [id],
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Récupère les résultats d'une veille
    pub async fn get_watch_results(&self, watch_id: i64) -> Result<Vec<WatchResult>, String> {
        let db = self.db.lock().await;
        let mut stmt = db.conn.prepare(
            "SELECT id, watch_id, title, url, content, translated_content, source_name, published_at, fetched_at, is_processed, metadata 
             FROM watch_results 
             WHERE watch_id = ? 
             ORDER BY fetched_at DESC"
        ).map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        let mut rows = stmt.query([watch_id]).map_err(|e| e.to_string())?;

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let result = WatchResult {
                id: row.get(0).ok(),
                watch_id: row.get(1).unwrap_or_default(),
                title: row.get(2).unwrap_or_default(),
                url: row.get(3).unwrap_or_default(),
                content: row.get(4).unwrap_or_default(),
                translated_content: row.get(5).ok(),
                source_name: row.get(6).unwrap_or_default(),
                published_at: row.get(7).ok(),
                fetched_at: row.get(8).ok(),
                is_processed: row.get(9).unwrap_or(false),
                metadata: row.get(10).ok(),
                entities: None,
            };
            results.push(result);
        }

        Ok(results)
    }

    /// Sauvegarde un résultat de veille
    pub async fn save_watch_result(&self, result: WatchResult) -> Result<i64, String> {
        let db = self.db.lock().await;
        let metadata_json = result.metadata.map(|m| serde_json::to_string(&m).unwrap_or_default())
            .unwrap_or_default();
        let now = chrono::Utc::now().to_rfc3339();

        let row_id = db.conn.execute(
            "INSERT INTO watch_results (watch_id, title, url, content, translated_content, source_name, published_at, fetched_at, is_processed, metadata) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                result.watch_id,
                result.title,
                result.url,
                result.content,
                result.translated_content.unwrap_or_default(),
                result.source_name,
                result.published_at.unwrap_or_default(),
                now,
                false,
                metadata_json,
            ),
        ).map_err(|e| e.to_string())?;

        Ok(row_id as i64)
    }

    /// Marque un résultat comme traité
    pub async fn mark_result_processed(&self, result_id: i64) -> Result<(), String> {
        let db = self.db.lock().await;
        db.conn.execute(
            "UPDATE watch_results SET is_processed = 1 WHERE id = ?",
            [result_id],
        ).map_err(|e| e.to_string())?;

        Ok(())
    }
}

/// Commandes Tauri pour les veilles


#[allow(dead_code)]
pub async fn get_watches(
    
        
        
            
        state: &crate::AppState) -> Result<Vec<Watch>, String> {
    let watch_manager = state.entity_manager.lock().await;
    watch_manager.get_watches()
}


#[allow(dead_code)]
pub async fn add_watch(
    
        
        
            
        state: &crate::AppState, watch: Watch) -> Result<i64, String> {
    let watch_manager = state.entity_manager.lock().await;
    watch_manager.add_watch(watch)
}


#[allow(dead_code)]
pub async fn update_watch(
    
        
        
            
        state: &crate::AppState, watch: Watch) -> Result<(), String> {
    let watch_manager = state.entity_manager.lock().await;
    watch_manager.update_watch(watch)
}


#[allow(dead_code)]
pub async fn delete_watch(
    
        
        
            
        state: &crate::AppState, id: i64) -> Result<(), String> {
    let watch_manager = state.entity_manager.lock().await;
    watch_manager.delete_watch(id)
}


#[allow(dead_code)]
pub async fn get_watch_results(
    
        
        
            
        state: &crate::AppState, watch_id: i64) -> Result<Vec<WatchResult>, String> {
    let watch_manager = state.entity_manager.lock().await;
    watch_manager.get_watch_results(watch_id)
}
