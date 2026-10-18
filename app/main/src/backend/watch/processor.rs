//! Module de traitement des résultats de veille
//! Gère le NER, la cotation et la capitalisation

use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use log::{info, error, warn};

use crate::backend::db::Database;
use super::{WatchResult, ProcessedWatchResult, EventGroup, ApiClient, WatchManager};

/// Processeur de résultats de veille
pub struct WatchProcessor {
    db: Arc<Mutex<Database>>,
    api_client: ApiClient,
    watch_manager: Arc<Mutex<WatchManager>>,
}

impl WatchProcessor {
    pub fn new(
        db: Arc<Mutex<Database>>,
        api_client: ApiClient,
        watch_manager: Arc<Mutex<WatchManager>>,
    ) -> Self {
        Self {
            db,
            api_client,
            watch_manager,
        }
    }

    /// Traite tous les résultats non traités
    pub async fn process_unprocessed_results(&self) -> Result<Vec<ProcessedWatchResult>, String> {
        let db = self.db.lock().unwrap();
        let unprocessed_results = db.get_unprocessed_watch_results()?;

        info!("Traitement de {} résultats de veille non traités", unprocessed_results.len());

        let mut processed_results = Vec::new();

        for result in unprocessed_results {
            // Convertir WatchResult en ProcessedWatchResult
            let processed = self.process_result(result).await?;
            
            // Sauvegarder le résultat traité
            self.save_processed_result(&processed).await?;
            
            // Marquer le résultat comme traité
            if let Some(id) = processed.id {
                db.mark_watch_result_processed(id)?;
            }
            
            processed_results.push(processed);
        }

        Ok(processed_results)
    }

    /// Traite un résultat individuel
    async fn process_result(&self, result: WatchResult) -> Result<ProcessedWatchResult, String> {
        // Extraire les entités avec NER
        let entities = self.extract_entities(&result).await?;

        // Obtenir la cotation de la source
        let source_rating = self.get_source_rating(&result.source_name).await?;

        // Proposer une cotation pour l'information
        let info_rating = self.propose_info_rating(&result, &entities).await?;

        // Créer le résultat traité
        Ok(ProcessedWatchResult {
            id: result.id.unwrap_or(0),
            watch_id: result.watch_id,
            title: result.title,
            url: result.url,
            content: result.content,
            translated_content: result.translated_content,
            source_name: result.source_name,
            published_at: result.published_at,
            fetched_at: result.fetched_at,
            entities,
            proposed_rating: format!("{}{}", source_rating, info_rating),
            source_rating,
            info_rating,
        })
    }

    /// Extrait les entités d'un résultat de veille
    async fn extract_entities(&self, result: &WatchResult) -> Result<Vec<super::EntityReference>, String> {
        // Utiliser le contenu traduit si disponible, sinon le contenu original
        let content = result.translated_content.as_deref().unwrap_or(&result.content);

        // Appeler l'API Mistral pour le NER
        let entities = self.api_client.extract_entities(content).await?;

        // Enrichir les entités avec des informations supplémentaires
        let mut enriched_entities = Vec::new();

        for entity in entities {
            // Vérifier si l'entité existe déjà dans la base
            let db = self.db.lock().unwrap();
            let existing_entity = db.get_entity_by_name(&entity.name, &entity.entity_type)?;

            let enriched_entity = super::EntityReference {
                entity_type: entity.entity_type,
                entity_id: existing_entity.map(|e| e.get("id").and_then(|v| v.as_i64()).unwrap_or(0)),
                name: entity.name,
                confidence: entity.confidence,
                matched: existing_entity.is_some(),
            };

            enriched_entities.push(enriched_entity);
        }

        Ok(enriched_entities)
    }

    /// Obtient la cotation de la source
    async fn get_source_rating(&self, source_name: &str) -> Result<String, String> {
        // Cotations par défaut pour les sources connues
        let source_ratings: std::collections::HashMap<&str, &str> = [
            ("Google Actualités", "B"),
            ("Qwant Actualités", "B"),
            ("Yandex Actualités", "C"),
            ("Twitter/X", "C"),
            ("Telegram", "D"),
            ("Discord", "D"),
            ("TikTok", "D"),
            ("ВКонтакте", "C"),
            ("BBC", "A"),
            ("Le Monde", "A"),
            ("AFP", "A"),
            ("Reuters", "A"),
            ("Wikipedia", "B"),
        ]
        .iter()
        .cloned()
        .collect();

        // Retourner la cotation par défaut ou "D" si la source est inconnue
        Ok(source_ratings.get(source_name).cloned().unwrap_or("D").to_string())
    }

    /// Propose une cotation pour l'information
    async fn propose_info_rating(
        &self,
        result: &WatchResult,
        entities: &[super::EntityReference],
    ) -> Result<String, String> {
        // Obtenir la cotation de la source
        let source_rating = self.get_source_rating(&result.source_name).await?;

        // Utiliser l'API Mistral pour proposer une cotation
        // Note: Cela peut être coûteux en tokens, donc on utilise une logique simple pour l'instant
        
        // Logique simple de proposition de cotation
        // - Si la source est très fiable (A/B) et que le contenu est détaillé, crédibilité = 1 ou 2
        // - Si la source est moyennement fiable (C) et que le contenu est court, crédibilité = 3 ou 4
        // - Si la source est peu fiable (D/E/F), crédibilité = 4, 5 ou 6
        
        match source_rating.as_str() {
            "A" => {
                // Source très fiable
                if result.content.len() > 500 {
                    Ok("1".to_string()) // Confirmée
                } else if result.content.len() > 200 {
                    Ok("2".to_string()) // Probablement vraie
                } else {
                    Ok("3".to_string()) // Possiblement vraie
                }
            }
            "B" => {
                // Source généralement fiable
                if result.content.len() > 500 {
                    Ok("2".to_string()) // Probablement vraie
                } else if result.content.len() > 200 {
                    Ok("3".to_string()) // Possiblement vraie
                } else {
                    Ok("4".to_string()) // Douteuse
                }
            }
            "C" => {
                // Source assez fiable
                if result.content.len() > 500 {
                    Ok("3".to_string()) // Possiblement vraie
                } else {
                    Ok("4".to_string()) // Douteuse
                }
            }
            _ => {
                // Source peu fiable ou non évaluable
                Ok("4".to_string()) // Douteuse
            }
        }
    }

    /// Sauvegarde un résultat traité
    async fn save_processed_result(&self, result: &ProcessedWatchResult) -> Result<(), String> {
        // Pour l'instant, on ne sauvegarde pas les résultats traités dans une table séparée
        // Ils sont simplement marqués comme traités dans la table watch_results
        // Dans une version future, on pourrait créer une table dedicated
        Ok(())
    }

    /// Regroupe les résultats par événements
    pub fn group_by_event(results: Vec<ProcessedWatchResult>) -> Vec<EventGroup> {
        use std::collections::HashMap;

        let mut groups: HashMap<String, EventGroup> = HashMap::new();

        for result in results {
            // Utiliser le titre comme clé d'événement (simplifié)
            // En production, utiliser une analyse sémantique pour regrouper les événements
            let event_key = result.title.clone();

            if let Some(group) = groups.get_mut(&event_key) {
                group.results.push(result);
            } else {
                let mut group = EventGroup {
                    event: event_key.clone(),
                    date: result.published_at.clone(),
                    results: vec![result],
                    proposed_rating: result.proposed_rating.clone(),
                };
                
                // Calculer la cotation moyenne du groupe
                group.proposed_rating = Self::calculate_average_rating(&group.results);
                
                groups.insert(event_key, group);
            }
        }

        groups.into_values().collect()
    }

    /// Calcule la cotation moyenne d'un groupe de résultats
    fn calculate_average_rating(results: &[ProcessedWatchResult]) -> String {
        if results.is_empty() {
            return "D3".to_string();
        }

        // Compter les occurrences de chaque cotation
        use std::collections::HashMap;
        let mut rating_counts: HashMap<String, usize> = HashMap::new();

        for result in results {
            *rating_counts.entry(result.proposed_rating.clone()).or_insert(0) += 1;
        }

        // Trouver la cotation la plus fréquente
        let most_common = rating_counts.into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(rating, _)| rating)
            .unwrap_or("D3".to_string());

        most_common
    }
}

/// Commandes Tauri pour le traitement des résultats


#[tauri::command]
pub fn get_processed_watch_results(
    
        
        
            
        state: tauri::State<crate::AppState>) -> Result<Vec<ProcessedWatchResult>, String> {
    let db = state.db.lock().unwrap();
    let unprocessed_results = db.get_unprocessed_watch_results()?;

    // Pour l'instant, on retourne les résultats non traités
    // Dans une version future, on retournera les résultats traités
    let mut processed_results = Vec::new();

    for result in unprocessed_results {
        processed_results.push(ProcessedWatchResult {
            id: result.id.unwrap_or(0),
            watch_id: result.watch_id,
            title: result.title,
            url: result.url,
            content: result.content,
            translated_content: result.translated_content,
            source_name: result.source_name,
            published_at: result.published_at,
            fetched_at: result.fetched_at,
            entities: result.entities.unwrap_or_default(),
            proposed_rating: "B2".to_string(), // Cotation par défaut
            source_rating: "B".to_string(),
            info_rating: "2".to_string(),
        });
    }

    Ok(processed_results)
}


#[tauri::command]
pub fn process_watch_results(
    
        
        
            
        state: tauri::State<crate::AppState>) -> Result<Vec<ProcessedWatchResult>, String> {
    let db = Arc::clone(&state.db);
    let tor_client = state.tor_manager.lock().unwrap();
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().unwrap().shared.api.mistral_key.clone(),
    );
    let watch_manager = Arc::clone(&state.entity_manager);

    let processor = WatchProcessor::new(db, api_client, watch_manager);

    // Exécuter le traitement en arrière-plan
    tokio::spawn(async move {
        if let Err(e) = processor.process_unprocessed_results().await {
            error!("Erreur lors du traitement des résultats de veille: {}", e);
        }
    });

    // Retourner les résultats traités (simulé pour l'instant)
    Ok(Vec::new())
}


#[tauri::command]
pub fn group_results_by_event(results: Vec<ProcessedWatchResult>) -> Vec<EventGroup> {
    WatchProcessor::group_by_event(results)
}
