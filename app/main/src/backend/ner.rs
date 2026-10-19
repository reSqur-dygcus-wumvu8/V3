use crate::AppState;
// Module de Reconnaissance d'Entités Nominales (NER)
// Gère l'extraction et la classification d'entités via Mistral API

use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use log::{info, error, warn};
use serde::{Serialize, Deserialize};

use crate::backend::{db::Database, tor::TorClient};
use super::watch::ApiClient;
use osint_shared::models::EntityType;

/// Type d'entité détectée
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectedEntityType {
    Individual,
    Organization,
    Location,
    Event,
    Object,
    Date,
    Time,
    Money,
    Percentage,
    Other,
}

impl DetectedEntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DetectedEntityType::Individual => "individual",
            DetectedEntityType::Organization => "organization",
            DetectedEntityType::Location => "location",
            DetectedEntityType::Event => "event",
            DetectedEntityType::Object => "object",
            DetectedEntityType::Date => "date",
            DetectedEntityType::Time => "time",
            DetectedEntityType::Money => "money",
            DetectedEntityType::Percentage => "percentage",
            DetectedEntityType::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "individual" | "person" | "personne" => Some(DetectedEntityType::Individual),
            "organization" | "org" | "organisation" => Some(DetectedEntityType::Organization),
            "location" | "loc" | "lieu" => Some(DetectedEntityType::Location),
            "event" | "événement" => Some(DetectedEntityType::Event),
            "object" | "objet" => Some(DetectedEntityType::Object),
            "date" => Some(DetectedEntityType::Date),
            "time" => Some(DetectedEntityType::Time),
            "money" | "argent" => Some(DetectedEntityType::Money),
            "percentage" | "pourcentage" => Some(DetectedEntityType::Percentage),
            _ => Some(DetectedEntityType::Other),
        }
    }

    /// Convertit en EntityType pour la base de connaissance
    pub fn to_entity_type(&self) -> EntityType {
        match self {
            DetectedEntityType::Individual => EntityType::Individual,
            DetectedEntityType::Organization => EntityType::Organization,
            DetectedEntityType::Location => EntityType::Location,
            DetectedEntityType::Event => EntityType::Event,
            DetectedEntityType::Object => EntityType::Object,
            _ => EntityType::Object, // Par défaut
        }
    }
}

/// Entité détectée par le NER
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedEntity {
    pub text: String,
    pub entity_type: DetectedEntityType,
    pub start_index: usize,
    pub end_index: usize,
    pub confidence: f64,
    pub context: Option<String>,
}

/// Résultat du NER pour un texte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NerResult {
    pub text: String,
    pub entities: Vec<DetectedEntity>,
    pub language: String,
    pub processed_at: String,
}

/// Gestionnaire du NER
pub struct NerManager {
    db: Arc<Mutex<Database>>,
    api_client: ApiClient,
}

impl NerManager {
    pub fn new(db: Arc<Mutex<Database>>, api_client: ApiClient) -> Self {
        Self { db, api_client }
    }

    /// Extrait les entités d'un texte
    pub async fn extract_entities(
        &self,
        text: &str,
        language: Option<&str>,
    ) -> Result<NerResult, String> {
        info!("Extraction d'entités du texte (langue: {})", language.unwrap_or("fr"));

        // Construire le prompt pour Mistral
        let prompt = self.build_ner_prompt(text, language)?;

        // Appeler Mistral API
        let response = self.api_client.call_mistral(&prompt, 2000, 0.1).await?;

        // Parser la réponse
        self.parse_ner_response(text, &response, language)
    }

    /// Construit le prompt pour le NER
    fn build_ner_prompt(&self, text: &str, language: Option<&str>) -> Result<String, String> {
        let lang = language.unwrap_or("français");
        let lang_code = match lang {
            "français" | "fr" => "fr",
            "english" | "en" => "en",
            "russian" | "ru" => "ru",
            _ => "fr",
        };

        Ok(format!(
            "Tu es un expert en analyse de texte pour la reconnaissance d'entités nominales (NER). \
             Analyse le texte suivant et extrait toutes les entités avec leur type, position et score de confiance. \
             \
             Texte à analyser (langue: {}): \
             \"\"\"{}\"\"\" \
             \
             Règle de sortie: \
             - Retourne UNIQUEMENT un JSON valide avec la structure suivante: \
             {{
                 \"entities\": [
                     {{
                         \"text\": \"Nom de l'entité\",
                         \"type\": \"individual|organization|location|event|object|date|time|money|percentage|other\",
                         \"start\": 0,
                         \"end\": 10,
                         \"confidence\": 0.95
                     }},
                     // ... plus d'entités
                 ]
             }} \
             \
             Types d'entités à identifier: \
             - individual: Noms de personnes (ex: Emmanuel Macron, Jean Dupont) \
             - organization: Noms d'organisations (ex: ONG, entreprises, gouvernements) \
             - location: Noms de lieux (villes, pays, adresses, coordonnées GPS) \
             - event: Noms d'événements (conflits, conférences, accidents) \
             - object: Noms d'objets (véhicules, armes, documents) \
             - date: Dates au format AAAA-MM-JJ ou autre format clair \
             - time: Heures au format HH:MM:SS ou autre format clair \
             - money: Montants d'argent (ex: 100$, 50€, 1 million) \
             - percentage: Pourcentages (ex: 25%, 50%) \
             - other: Autres entités non classées \
             \
             Consignes: \
             - La langue du texte est {}. \
             - Sois précis sur les positions (start et end sont des indices de caractères). \
             - Le score de confiance doit être entre 0 et 1. \
             - Ne retourne AUCUN autre texte, seulement le JSON.",
            lang, text
        ))
    }

    /// Parse la réponse de Mistral
    fn parse_ner_response(
        &self,
        original_text: &str,
        response: &str,
        language: Option<&str>,
    ) -> Result<NerResult, String> {
        // Nettoyer la réponse
        let json_str = response.trim();

        // Essayer de parser directement
        if let Ok(parsed) = serde_json::from_str::<NerResponse>(json_str) {
            let now = chrono::Utc::now().to_rfc3339();
            return Ok(NerResult {
                text: original_text.to_string(),
                entities: parsed.entities,
                language: language.unwrap_or("fr").to_string(),
                processed_at: now,
            });
        }

        // Si le parsing échoue, essayer d'extraire le JSON
        if let Some(start) = json_str.find('{') {
            if let Some(end) = json_str.rfind('}') {
                let json_part = &json_str[start..=end];
                if let Ok(parsed) = serde_json::from_str::<NerResponse>(json_part) {
                    let now = chrono::Utc::now().to_rfc3339();
                    return Ok(NerResult {
                        text: original_text.to_string(),
                        entities: parsed.entities,
                        language: language.unwrap_or("fr").to_string(),
                        processed_at: now,
                    });
                }
            }
        }

        // Si tout échoue, retourner une erreur
        Err(format!("Réponse Mistral invalide pour le NER: {}", json_str))
    }

    /// Extrait les entités et les lie à des entités existantes dans la base
    pub async fn extract_and_match_entities(
        &self,
        text: &str,
        language: Option<&str>,
    ) -> Result<Vec<MatchedEntity>, String> {
        // Extraire les entités
        let ner_result = self.extract_entities(text, language).await?;

        // Lier les entités aux entités existantes dans la base
        let mut matched_entities = Vec::new();

        for entity in ner_result.entities {
            let db = self.db.lock().await;
            
            // Rechercher des entités similaires dans la base
            let similar_entities = db.search_entities(
                &entity.text,
                entity.entity_type.to_entity_type(),
            )?;

            // Trouver la meilleure correspondance
            let best_match = similar_entities.into_iter()
                .max_by(|a, b| {
                    // Comparer par similarité de nom
                    let a_sim = text_similarity(&entity.text, &a.get("name").and_then(|v| v.as_str()).unwrap_or(""));
                    let b_sim = text_similarity(&entity.text, &b.get("name").and_then(|v| v.as_str()).unwrap_or(""));
                    a_sim.cmp(&b_sim)
                });

            let matched_entity = MatchedEntity {
                detected_entity: entity,
                matched_id: best_match.as_ref().and_then(|e| e.get("id").and_then(|v| v.as_i64())),
                matched_name: best_match.as_ref().and_then(|e| e.get("name").and_then(|v| v.as_str())).map(|s| s.to_string()),
                confidence: best_match.map(|_| 0.9).unwrap_or(0.0), // Confiance par défaut
                is_new: best_match.is_none(),
            };

            matched_entities.push(matched_entity);
        }

        Ok(matched_entities)
    }

    /// Extrait les relations entre entités dans un texte
    pub async fn extract_relations(
        &self,
        text: &str,
        entities: &[DetectedEntity],
    ) -> Result<Vec<EntityRelation>, String> {
        if entities.is_empty() {
            return Ok(Vec::new());
        }

        // Construire le prompt pour l'extraction de relations
        let prompt = self.build_relation_prompt(text, entities)?;

        // Appeler Mistral API
        let response = self.api_client.call_mistral(&prompt, 2000, 0.3).await?;

        // Parser la réponse
        self.parse_relation_response(&response)
    }

    /// Construit le prompt pour l'extraction de relations
    fn build_relation_prompt(
        &self,
        text: &str,
        entities: &[DetectedEntity],
    ) -> Result<String, String> {
        let entity_list: Vec<String> = entities.iter()
            .map(|e| format!("- {} ({})", e.text, e.entity_type.as_str()))
            .collect();
        
        let entity_json: Vec<serde_json::Value> = entities.iter()
            .map(|e| serde_json::json!({
                "name": e.text,
                "type": e.entity_type.as_str()
            }))
            .collect();

        Ok(format!(
            "Tu es un expert en analyse de relations entre entités. \
             Analyse le texte suivant et identifie les relations entre les entités détectées. \
             \
             Texte: \
             \"\"\"{}\"\"\" \
             \
             Entités détectées: \
             {} \
             \
             Règle de sortie: \
             - Retourne UNIQUEMENT un JSON valide avec la structure suivante: \
             {{
                 \"relations\": [
                     {{
                         \"source\": \"Nom de l'entité source\",
                         \"target\": \"Nom de l'entité cible\",
                         \"relation\": \"type de relation (ex: travaille pour, membre de, situé à, etc.)\",
                         \"confidence\": 0.85,
                         \"context\": \"Contexte de la relation\"
                     }},
                     // ... plus de relations
                 ]
             }} \
             \
             Consignes: \
             - Identifie les relations explicites et implicites. \
             - Les types de relation peuvent être: travaille pour, membre de, situé à, possède, parent de, associé à, etc. \
             - Le score de confiance doit être entre 0 et 1. \
             - Ne retourne AUCUN autre texte, seulement le JSON.",
            text, entity_list.join("\n")
        ))
    }

    /// Parse la réponse de relation
    fn parse_relation_response(&self, response: &str) -> Result<Vec<EntityRelation>, String> {
        let json_str = response.trim();

        if let Ok(parsed) = serde_json::from_str::<RelationResponse>(json_str) {
            return Ok(parsed.relations);
        }

        if let Some(start) = json_str.find('{') {
            if let Some(end) = json_str.rfind('}') {
                let json_part = &json_str[start..=end];
                if let Ok(parsed) = serde_json::from_str::<RelationResponse>(json_part) {
                    return Ok(parsed.relations);
                }
            }
        }

        Err(format!("Réponse Mistral invalide pour les relations: {}", json_str))
    }

    /// Sauvegarde les résultats du NER dans la base
    pub async fn save_ner_result(
        &self,
        text: &str,
        ner_result: &NerResult,
        source_id: Option<i64>,
    ) -> Result<i64, String> {
        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        // Sauvegarder le résultat du NER
        let ner_id = db.conn.execute(
            "INSERT INTO ner_results (text, language, processed_at, source_id) 
             VALUES (?, ?, ?, ?)",
            (text, ner_result.language.clone(), now.clone(), source_id.unwrap_or(0)),
        ).map_err(|e| e.to_string())?;

        // Sauvegarder les entités
        for entity in &ner_result.entities {
            db.conn.execute(
                "INSERT INTO ner_entities (ner_result_id, text, entity_type, start_index, end_index, confidence, context) 
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                (
                    ner_id,
                    entity.text.clone(),
                    entity.entity_type.as_str(),
                    entity.start_index as i64,
                    entity.end_index as i64,
                    entity.confidence,
                    entity.context.as_deref().unwrap_or(""),
                ),
            ).map_err(|e| e.to_string())?;
        }

        Ok(ner_id as i64)
    }
}

/// Entité avec correspondance dans la base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedEntity {
    pub detected_entity: DetectedEntity,
    pub matched_id: Option<i64>,
    pub matched_name: Option<String>,
    pub confidence: f64,
    pub is_new: bool,
}

/// Relation entre entités
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelation {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub confidence: f64,
    pub context: Option<String>,
}

/// Réponse du NER depuis Mistral
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NerResponse {
    pub entities: Vec<DetectedEntity>,
}

/// Réponse des relations depuis Mistral
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelationResponse {
    pub relations: Vec<EntityRelation>,
}

/// Calcule la similarité entre deux textes (implémentation simple)
fn text_similarity(a: &str, b: &str) -> f64 {
    // Implémentation simple basée sur la distance de Levenshtein
    let distance = levenshtein_distance(a, b);
    let max_len = a.len().max(b.len()) as f64;
    
    if max_len == 0.0 {
        1.0
    } else {
        1.0 - (distance as f64 / max_len)
    }
}

/// Distance de Levenshtein
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    
    let m = a_chars.len();
    let n = b_chars.len();
    
    let mut dp = vec![vec![0; n + 1]; m + 1];
    
    for i in 0..=m {
        dp[i][0] = i;
    }
    
    for j in 0..=n {
        dp[0][j] = j;
    }
    
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    
    dp[m][n]
}

/// Commandes Tauri pour le NER


#[allow(dead_code)]
pub async fn extract_entities(
    
    
        
        
            
        state: &crate::AppState,
    text: String,
    language: Option<String>,
) -> Result<NerResult, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );

    let ner_manager = NerManager::new(Arc::clone(&state.db), api_client);
    ner_manager.extract_entities(&text, language.as_deref()).await
}


#[allow(dead_code)]
pub async fn extract_and_match_entities(
    
    
        
        
            
        state: &crate::AppState,
    text: String,
    language: Option<String>,
) -> Result<Vec<MatchedEntity>, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );

    let ner_manager = NerManager::new(Arc::clone(&state.db), api_client);
    ner_manager.extract_and_match_entities(&text, language.as_deref()).await
}


#[allow(dead_code)]
pub async fn extract_relations(
    
    
        
        
            
        state: &crate::AppState,
    text: String,
    entities: Vec<DetectedEntity>,
) -> Result<Vec<EntityRelation>, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );

    let ner_manager = NerManager::new(Arc::clone(&state.db), api_client);
    ner_manager.extract_relations(&text, &entities).await
}


#[allow(dead_code)]
pub async fn save_ner_result(
    
    
        
        
            
        state: &crate::AppState,
    text: String,
    ner_result: NerResult,
    source_id: Option<i64>,
) -> Result<i64, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );

    let ner_manager = NerManager::new(Arc::clone(&state.db), api_client);
    ner_manager.save_ner_result(&text, &ner_result, source_id).await
}


#[allow(dead_code)]
pub async fn get_entity_types() -> Vec<String> {
    vec![
        DetectedEntityType::Individual.as_str().to_string(),
        DetectedEntityType::Organization.as_str().to_string(),
        DetectedEntityType::Location.as_str().to_string(),
        DetectedEntityType::Event.as_str().to_string(),
        DetectedEntityType::Object.as_str().to_string(),
        DetectedEntityType::Date.as_str().to_string(),
        DetectedEntityType::Time.as_str().to_string(),
        DetectedEntityType::Money.as_str().to_string(),
        DetectedEntityType::Percentage.as_str().to_string(),
        DetectedEntityType::Other.as_str().to_string(),
    ]
}
