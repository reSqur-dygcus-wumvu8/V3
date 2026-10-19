use crate::AppState;
// Module de découverte
// Gère le mode découverte avec Mistral API

use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use log::{info, error};

use crate::backend::{tor::TorClient, db::Database};
use super::watch::ApiClient;

/// Résultat d'une découverte
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryResult {
    pub summary: String,
    pub sources: Vec<DiscoveredSource>,
    pub keywords: Vec<String>,
}

/// Source découverte
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveredSource {
    pub name: String,
    pub url: String,
}

/// Gestionnaire de découverte
pub struct DiscoveryManager {
    api_client: ApiClient,
    db: Arc<Mutex<Database>>,
}

impl DiscoveryManager {
    pub async fn new(api_client: ApiClient, db: Arc<Mutex<Database>>) -> Self {
        Self { api_client, db }
    }

    /// Explore une thématique en profondeur
    pub async fn discover_topic(
        &self,
        topic: String,
        language: Option<String>,
        max_tokens: Option<u32>,
        temperature: Option<f64>,
    ) -> Result<DiscoveryResult, String> {
        info!("Démarrage de la découverte pour la thématique: {}", topic);

        // Construire le prompt pour Mistral
        let prompt = self.build_discovery_prompt(&topic, language.as_deref())?;

        // Appeler Mistral API
        let response = self.api_client.call_mistral(
            &prompt,
            max_tokens.unwrap_or(2000),
            temperature.unwrap_or(0.7),
        ).await?;

        // Parser la réponse
        self.parse_discovery_response(&response)
    }

    /// Construit le prompt pour la découverte
    fn build_discovery_prompt(&self, topic: &str, language: Option<&str>) -> Result<String, String> {
        let lang = language.unwrap_or("français");
        
        Ok(format!(
            "Tu es un analyste en renseignement spécialisé dans l'exploration de thématiques complexes. \
             Explore la thématique suivante en profondeur : \"{topic}\". \
             \
             Réponds UNIQUEMENT avec un JSON structuré comme suit : \
             {{
                 \"summary\": \"Une synthèse structurée en {} de 500 à 1000 mots sur la thématique, \
                              incluant les acteurs principaux, les enjeux, les tendances récentes et les perspectives futures. \
                              Ne mentionne pas que tu es un modèle d'IA.\",
                 \"sources\": [
                     {{
                         \"name\": \"Nom de la source 1\",
                         \"url\": \"https://exemple.com/source1\"
                     }},
                     {{
                         \"name\": \"Nom de la source 2\",
                         \"url\": \"https://exemple.com/source2\"
                     }}
                 ],
                 \"keywords\": [\"mot-clé 1\", \"mot-clé 2\", \"mot-clé 3\"]
             }} \
             \
             Consignes : \
             - La synthèse doit être en {}. \
             - Les sources doivent être fiables et pertinentes pour la thématique. \
             - Les mots-clés doivent être utiles pour des recherches futures. \
             - Ne fais pas de référence à ce format dans ta réponse, réponds uniquement avec le JSON.",
            topic, lang
        ))
    }

    /// Parse la réponse de Mistral
    fn parse_discovery_response(&self, response: &str) -> Result<DiscoveryResult, String> {
        // Nettoyer la réponse pour extraire le JSON
        let json_str = response.trim();

        // Essayer de parser directement
        if let Ok(result) = serde_json::from_str::<DiscoveryResult>(json_str) {
            return Ok(result);
        }

        // Si le parsing échoue, essayer d'extraire le JSON du texte
        // (Mistral peut parfois ajouter du texte avant/après le JSON)
        if let Some(start) = json_str.find('{') {
            if let Some(end) = json_str.rfind('}') {
                let json_part = &json_str[start..=end];
                if let Ok(result) = serde_json::from_str::<DiscoveryResult>(json_part) {
                    return Ok(result);
                }
            }
        }

        // Si tout échoue, retourner une erreur
        Err(format!("Réponse Mistral invalide: {}", json_str))
    }

    /// Ajoute les sources découvertes aux veilles
    pub async fn add_discovered_sources_to_watch(
        &self,
        sources: Vec<DiscoveredSource>,
        watch_name: String,
        query: String,
        language: String,
        frequency: u32,
    ) -> Result<i64, String> {
        // Pour l'instant, on crée une nouvelle veille avec les sources
        // Dans une version future, on permettra d'ajouter les sources à une veille existante
        
        // Extraire les IDs des sources (ex: google_news, qwant, etc.)
        let mut source_ids = Vec::new();
        for source in &sources {
            // Mapper le nom de la source à un ID connu
            let source_id = match source.name.to_lowercase().as_str() {
                "google actualités" | "google news" => "google_news",
                "qwant" | "qwant actualités" => "qwant",
                "yandex" | "yandex actualités" => "yandex",
                "twitter" | "twitter/x" => "twitter",
                "telegram" => "telegram",
                _ => continue, // Ignorer les sources non reconnues
            };
            source_ids.push(source_id.to_string());
        }

        if source_ids.is_empty() {
            return Err("Aucune source reconnue dans les résultats".to_string());
        }

        // Créer une nouvelle veille
        let db = self.db.lock().await;
        let sources_json = serde_json::to_string(&source_ids).unwrap_or_default();
        let now = chrono::Utc::now().to_rfc3339();

        let watch_id = db.conn.execute(
            "INSERT INTO watches (name, query, language, sources, frequency_minutes, is_active, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (
                watch_name,
                query,
                language,
                sources_json,
                frequency,
                true,
                now.clone(),
                now,
            ),
        ).map_err(|e| e.to_string())?;

        Ok(watch_id as i64)
    }
}

/// Commandes Tauri pour le mode découverte


#[allow(dead_code)]
pub async fn discover_topic(
    
    
        
        
            
        state: &crate::AppState,
    topic: String,
    language: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f64>,
) -> Result<DiscoveryResult, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );
    
    let discovery_manager = DiscoveryManager::new(api_client, Arc::clone(&state.db));
    discovery_manager.discover_topic(topic, language, max_tokens, temperature).await
}


#[allow(dead_code)]
pub async fn add_discovered_sources_to_watch(
    
    
        
        
            
        state: &crate::AppState,
    sources: Vec<DiscoveredSource>,
    watch_name: String,
    query: String,
    language: String,
    frequency: u32,
) -> Result<i64, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );
    
    let discovery_manager = DiscoveryManager::new(api_client, Arc::clone(&state.db));
    discovery_manager.add_discovered_sources_to_watch(sources, watch_name, query, language, frequency).await
}
