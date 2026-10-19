use osint_shared::models::EntityType;
use crate::AppState;
// Module de capitalisation des informations
// Gère la transformation des résultats de veille en entités de la base de connaissance

use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use log::{info, error, warn};
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::backend::{db::Database, tor::TorClient};
use super::{watch::{WatchResult, ProcessedWatchResult, EventGroup, EntityReference}, 
          ner::{NerManager, DetectedEntity, MatchedEntity},
          entities::{EntityManager, EntityType, Individual, Organization, Event, Location, Object, Source},
          cotations::{CotationManager, OtanCotation},
          dedup::DeduplicationManager,
          api::MistralApiClient};

/// Information capitalisée
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalizedInfo {
    pub id: Option<i64>,
    pub watch_result_id: i64,
    pub entity_type: EntityType,
    pub entity_id: i64,
    pub title: String,
    pub content: String,
    pub source_name: String,
    pub source_url: Option<String>,
    pub cotation: OtanCotation,
    pub is_validated: bool,
    pub validated_by: Option<i64>,
    pub validated_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Gestionnaire de capitalisation
pub struct CapitalizationManager {
    db: Arc<Mutex<Database>>,
    tor_client: TorClient,
    api_client: Option<MistralApiClient>,
}

impl CapitalizationManager {
    pub fn new(db: Arc<Mutex<Database>>, tor_client: TorClient) -> Self {
        Self {
            db,
            tor_client,
            api_client: None,
        }
    }

    pub fn with_api_client(mut self, api_client: MistralApiClient) -> Self {
        self.api_client = Some(api_client);
        self
    }

    /// Capitalise un résultat de veille
    pub async fn capitalize_watch_result(
        &self,
        watch_result: &ProcessedWatchResult,
        user_id: Option<i64>,
    ) -> Result<CapitalizedInfo, String> {
        info!("Capitalisation du résultat de veille: {} (ID: {})", watch_result.title, watch_result.id);

        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        // 1. Extraire les entités du résultat
        let entities = self.extract_entities_from_result(watch_result).await?;

        // 2. Lier les entités aux entités existantes
        let matched_entities = self.match_entities(&entities).await?;

        // 3. Créer ou mettre à jour les entités
        let entity_ids = self.create_or_update_entities(&matched_entities, watch_result, user_id).await?;

        // 4. Créer l'information capitalisée
        // Pour l'instant, on lie à la première entité trouvée
        let entity_id = entity_ids.first().cloned().unwrap_or(0);
        let entity_type = if entity_id > 0 {
            matched_entities.first().map(|e| e.detected_entity.entity_type.to_entity_type())
                .unwrap_or(EntityType::Source)
        } else {
            EntityType::Source
        };

        // Sauvegarder dans la base
        let capitalized_id = db.conn.execute(
            "INSERT INTO capitalized_infos (watch_result_id, entity_type, entity_id, title, content, source_name, source_url, cotation, is_validated, validated_by, validated_at, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                watch_result.id,
                entity_type.as_str(),
                entity_id,
                watch_result.title.clone(),
                watch_result.content.clone(),
                watch_result.source_name.clone(),
                Some(watch_result.url.clone()),
                watch_result.proposed_rating.clone(),
                false, // Non validé par défaut
                user_id,
                None as Option<String>,
                now.clone(),
                now.clone(),
            ),
        ).map_err(|e| e.to_string())?;

        // 5. Marquer le résultat comme capitalisé
        db.conn.execute(
            "UPDATE watch_results SET is_capitalized = 1 WHERE id = ?",
            [watch_result.id],
        ).map_err(|e| e.to_string())?;

        // 6. Sauvegarder les liens entre entités
        self.save_entity_relations(&matched_entities, watch_result.id).await?;

        Ok(CapitalizedInfo {
            id: Some(capitalized_id as i64),
            watch_result_id: watch_result.id,
            entity_type,
            entity_id,
            title: watch_result.title.clone(),
            content: watch_result.content.clone(),
            source_name: watch_result.source_name.clone(),
            source_url: Some(watch_result.url.clone()),
            cotation: OtanCotation::from_str(&watch_result.proposed_rating)?,
            is_validated: false,
            validated_by: user_id,
            validated_at: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Capitalise un groupe d'événements
    pub async fn capitalize_event_group(
        &self,
        event_group: &EventGroup,
        user_id: Option<i64>,
    ) -> Result<Vec<CapitalizedInfo>, String> {
        info!("Capitalisation du groupe d'événements: {}", event_group.event);

        let mut capitalized_infos = Vec::new();

        for result in &event_group.results {
            let capitalized = self.capitalize_watch_result(result, user_id).await?;
            capitalized_infos.push(capitalized);
        }

        Ok(capitalized_infos)
    }

    /// Extrait les entités d'un résultat de veille
    async fn extract_entities_from_result(
        &self,
        result: &ProcessedWatchResult,
    ) -> Result<Vec<DetectedEntity>, String> {
        // Si des entités sont déjà présentes, les retourner
        if !result.entities.is_empty() {
            return Ok(result.entities.iter().map(|e| DetectedEntity {
                text: e.name.clone(),
                entity_type: super::ner::DetectedEntityType::from_str(&e.entity_type)
                    .unwrap_or(super::ner::DetectedEntityType::Other),
                start_index: 0,
                end_index: e.name.len(),
                confidence: e.confidence,
                context: None,
            }).collect());
        }

        // Sinon, extraire les entités du contenu
        let api_client = self.api_client.as_ref()
            .ok_or("ApiClient non initialisé")?;

        let ner_manager = NerManager::new(Arc::clone(&self.db), super::watch::ApiClient::new(
            self.tor_client.create_tor_client()?,
            api_client.api_key.clone(),
        ));

        let content = result.translated_content.as_deref().unwrap_or(&result.content);
        let ner_result = ner_manager.extract_entities(content, Some("fr")).await?;

        Ok(ner_result.entities)
    }

    /// Lie les entités aux entités existantes
    async fn match_entities(
        &self,
        entities: &[DetectedEntity],
    ) -> Result<Vec<MatchedEntity>, String> {
        let api_client = self.api_client.as_ref()
            .ok_or("ApiClient non initialisé")?;

        let ner_manager = NerManager::new(Arc::clone(&self.db), super::watch::ApiClient::new(
            self.tor_client.create_tor_client()?,
            api_client.api_key.clone(),
        ));

        ner_manager.extract_and_match_entities(
            &entities.iter().map(|e| e.text.clone()).collect::<Vec<_>>().join(" "),
            Some("fr"),
        ).await
    }

    /// Crée ou met à jour les entités
    async fn create_or_update_entities(
        &self,
        matched_entities: &[MatchedEntity],
        result: &ProcessedWatchResult,
        user_id: Option<i64>,
    ) -> Result<Vec<i64>, String> {
        let entity_manager = EntityManager::new(Arc::clone(&self.db));
        let mut entity_ids = Vec::new();

        for matched_entity in matched_entities {
            if matched_entity.is_new {
                // Créer une nouvelle entité
                let entity_id = match matched_entity.detected_entity.entity_type {
                    super::ner::DetectedEntityType::Individual => {
                        entity_manager.create_individual(
                            super::entities::IndividualType::Real,
                            None,
                            None,
                            matched_entity.detected_entity.text.clone(),
                            None,
                            Some(format!("Créé depuis la capitalisation de: {}", result.title)),
                            None,
                            None,
                            None,
                            None,
                            None,
                        ).await.map_err(|e| e.to_string())?
                    }
                    super::ner::DetectedEntityType::Organization => {
                        entity_manager.create_organization(
                            matched_entity.detected_entity.text.clone(),
                            super::entities::OrganizationType::Autre,
                            None,
                            Some(format!("Créé depuis la capitalisation de: {}", result.title)),
                            None,
                            None,
                            None,
                            None,
                        ).await.map_err(|e| e.to_string())?
                    }
                    super::ner::DetectedEntityType::Location => {
                        entity_manager.create_location(
                            super::entities::LocationType::Real,
                            matched_entity.detected_entity.text.clone(),
                            None,
                            None,
                            None,
                            Some(format!("Créé depuis la capitalisation de: {}", result.title)),
                            None,
                        ).await.map_err(|e| e.to_string())?
                    }
                    super::ner::DetectedEntityType::Event => {
                        entity_manager.create_event(
                            matched_entity.detected_entity.text.clone(),
                            result.published_at.clone(),
                            None,
                            None,
                            Some(format!("Créé depuis la capitalisation de: {}", result.title)),
                            None,
                            None,
                        ).await.map_err(|e| e.to_string())?
                    }
                    super::ner::DetectedEntityType::Object => {
                        entity_manager.create_object(
                            super::entities::ObjectType::Autre,
                            matched_entity.detected_entity.text.clone(),
                            None,
                            Some(format!("Créé depuis la capitalisation de: {}", result.title)),
                            None,
                        ).await.map_err(|e| e.to_string())?
                    }
                    _ => {
                        // Créer une entité générique
                        entity_manager.create_entity(
                            EntityType::Object,
                            matched_entity.detected_entity.text.clone(),
                            Some(format!("Créé depuis la capitalisation de: {}", result.title)),
                            None,
                        ).await.map_err(|e| e.to_string())?
                    }
                };

                entity_ids.push(entity_id);

                // Lier la nouvelle entité au résultat de veille
                self.link_entity_to_watch_result(
                    matched_entity.detected_entity.entity_type.to_entity_type(),
                    entity_id,
                    result.id,
                ).await?;
            } else if let Some(existing_id) = matched_entity.matched_id {
                // Utiliser l'entité existante
                entity_ids.push(existing_id);

                // Mettre à jour l'entité existante avec les nouvelles informations
                self.update_entity_from_watch_result(
                    matched_entity.detected_entity.entity_type.to_entity_type(),
                    existing_id,
                    result,
                ).await?;
            }
        }

        Ok(entity_ids)
    }

    /// Lie une entité à un résultat de veille
    async fn link_entity_to_watch_result(
        &self,
        entity_type: EntityType,
        entity_id: i64,
        watch_result_id: i64,
    ) -> Result<(), String> {
        let db = self.db.lock().await;

        db.conn.execute(
            "INSERT INTO watch_result_entities (watch_result_id, entity_type, entity_id) 
             VALUES (?, ?, ?)",
            (watch_result_id, entity_type.as_str(), entity_id),
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Met à jour une entité avec les informations d'un résultat de veille
    async fn update_entity_from_watch_result(
        &self,
        entity_type: EntityType,
        entity_id: i64,
        result: &ProcessedWatchResult,
    ) -> Result<(), String> {
        let entity_manager = EntityManager::new(Arc::clone(&self.db));

        // Récupérer l'entité existante
        let entity = entity_manager.get_entity(entity_type.clone(), entity_id).await.map_err(|e| e.to_string())?
            .ok_or("Entité non trouvée")?;

        // Mettre à jour avec les nouvelles informations
        let mut updates = serde_json::Map::new();
        
        // Ajouter le contenu du résultat comme commentaire
        let current_comments = entity.get("comments").and_then(|v| v.as_str()).unwrap_or_default();
        let new_comments = format!(
            "{}\n\n[Capitalisation automatique - {}] {}",
            current_comments,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            result.content
        );

        updates.insert("comments".to_string(), json!(new_comments));

        // Mettre à jour la cotation si la nouvelle cotation est meilleure
        if let Some(current_cotation_str) = entity.get("cotation").and_then(|v| v.as_str()) {
            if let Ok(current_cotation) = OtanCotation::from_str(current_cotation_str) {
                if let Ok(new_cotation) = OtanCotation::from_str(&result.proposed_rating) {
                    // Comparer les cotations
                    if new_cotation.compare(&current_cotation) == std::cmp::Ordering::Greater {
                        updates.insert("cotation".to_string(), json!(result.proposed_rating.clone()));
                    }
                }
            }
        } else {
            updates.insert("cotation".to_string(), json!(result.proposed_rating.clone()));
        }

        entity_manager.update_entity(entity_type, entity_id, serde_json::Value::Object(updates)).await?;

        Ok(())
    }

    /// Sauvegarde les relations entre entités
    async fn save_entity_relations(
        &self,
        matched_entities: &[MatchedEntity],
        watch_result_id: i64,
    ) -> Result<(), String> {
        // Pour l'instant, on ne sauvegarde pas les relations
        // En production, extraire les relations depuis le contenu
        Ok(())
    }

    /// Valide une information capitalisée
    pub async fn validate_capitalized_info(
        &self,
        capitalized_info_id: i64,
        user_id: i64,
        is_valid: bool,
        comments: Option<String>,
    ) -> Result<(), String> {
        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        if is_valid {
            db.conn.execute(
                "UPDATE capitalized_infos SET is_validated = 1, validated_by = ?, validated_at = ?, comments = ? WHERE id = ?",
                (user_id, now, comments, capitalized_info_id),
            ).map_err(|e| e.to_string())?;
        } else {
            // Si non valide, marquer comme non validé et ajouter les commentaires
            db.conn.execute(
                "UPDATE capitalized_infos SET is_validated = 0, validated_by = ?, validated_at = ?, comments = ? WHERE id = ?",
                (user_id, now, comments, capitalized_info_id),
            ).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Récupère les informations capitalisées non validées
    pub async fn get_unvalidated_capitalized_infos(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<CapitalizedInfo>, String> {
        let db = self.db.lock().await;

        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();

        let mut stmt = db.conn.prepare(&format!(
            "SELECT id, watch_result_id, entity_type, entity_id, title, content, source_name, source_url, cotation, is_validated, validated_by, validated_at, created_at, updated_at 
             FROM capitalized_infos 
             WHERE is_validated = 0 
             ORDER BY created_at DESC{}",
            limit_clause
        )).map_err(|e| e.to_string())?;

        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

        let mut results = Vec::new();

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let cotation_str: String = row.get(8).unwrap_or_default();
            let cotation = OtanCotation::from_str(&cotation_str).unwrap_or(OtanCotation::new('D', '4')?);

            results.push(CapitalizedInfo {
                id: row.get(0).ok(),
                watch_result_id: row.get(1).unwrap_or_default(),
                entity_type: EntityType::from_str(&row.get::<_, String>(2).unwrap_or_default())
                    .unwrap_or(EntityType::Source),
                entity_id: row.get(3).unwrap_or_default(),
                title: row.get(4).unwrap_or_default(),
                content: row.get(5).unwrap_or_default(),
                source_name: row.get(6).unwrap_or_default(),
                source_url: row.get(7).ok(),
                cotation,
                is_validated: row.get(9).unwrap_or(false),
                validated_by: row.get(10).ok(),
                validated_at: row.get(11).ok(),
                created_at: row.get(12).unwrap_or_default(),
                updated_at: row.get(13).unwrap_or_default(),
            });
        }

        Ok(results)
    }

    /// Récupère les informations capitalisées par une veille
    pub async fn get_capitalized_infos_by_watch(
        &self,
        watch_id: i64,
    ) -> Result<Vec<CapitalizedInfo>, String> {
        let db = self.db.lock().await;

        let mut stmt = db.conn.prepare(
            "SELECT ci.id, ci.watch_result_id, ci.entity_type, ci.entity_id, ci.title, ci.content, ci.source_name, ci.source_url, ci.cotation, ci.is_validated, ci.validated_by, ci.validated_at, ci.created_at, ci.updated_at 
             FROM capitalized_infos ci 
             JOIN watch_results wr ON ci.watch_result_id = wr.id 
             WHERE wr.watch_id = ? 
             ORDER BY ci.created_at DESC"
        ).map_err(|e| e.to_string())?;

        let mut rows = stmt.query([watch_id]).map_err(|e| e.to_string())?;

        let mut results = Vec::new();

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let cotation_str: String = row.get(8).unwrap_or_default();
            let cotation = OtanCotation::from_str(&cotation_str).unwrap_or(OtanCotation::new('D', '4')?);

            results.push(CapitalizedInfo {
                id: row.get(0).ok(),
                watch_result_id: row.get(1).unwrap_or_default(),
                entity_type: EntityType::from_str(&row.get::<_, String>(2).unwrap_or_default())
                    .unwrap_or(EntityType::Source),
                entity_id: row.get(3).unwrap_or_default(),
                title: row.get(4).unwrap_or_default(),
                content: row.get(5).unwrap_or_default(),
                source_name: row.get(6).unwrap_or_default(),
                source_url: row.get(7).ok(),
                cotation,
                is_validated: row.get(9).unwrap_or(false),
                validated_by: row.get(10).ok(),
                validated_at: row.get(11).ok(),
                created_at: row.get(12).unwrap_or_default(),
                updated_at: row.get(13).unwrap_or_default(),
            });
        }

        Ok(results)
    }

    /// Capitalise automatiquement depuis un résultat de veille
    pub async fn auto_capitalize(
        &self,
        watch_result: &WatchResult,
        user_id: Option<i64>,
    ) -> Result<CapitalizedInfo, String> {
        // Traiter le résultat pour obtenir un ProcessedWatchResult
        let api_client = self.api_client.as_ref()
            .ok_or("ApiClient non initialisé")?;

        let processor = super::watch::WatchProcessor::new(
            Arc::clone(&self.db),
            super::watch::ApiClient::new(
                self.tor_client.create_tor_client()?,
                api_client.api_key.clone(),
            ),
            Arc::new(super::watch::WatchManager::new(
                Arc::clone(&self.db),
                self.tor_client.create_tor_client()?,
                super::watch::ApiClient::new(
                    self.tor_client.create_tor_client()?,
                    api_client.api_key.clone(),
                ),
            )),
        );

        let processed = processor.process_result(watch_result.clone()).await?;

        // Capitaliser le résultat traité
        self.capitalize_watch_result(&processed, user_id).await
    }
}

/// Commandes Tauri pour la capitalisation


#[allow(dead_code)]
pub async fn capitalize_watch_result(
    
    
        
        
            
        state: &crate::AppState,
    watch_result_id: i64,
    user_id: Option<i64>,
) -> Result<CapitalizedInfo, String> {
    let db = state.db.lock().await;
    
    let mut stmt = db.conn.prepare(
        "SELECT id, watch_id, title, url, content, translated_content, source_name, published_at, fetched_at, is_processed, metadata 
         FROM watch_results 
         WHERE id = ?"
    ).map_err(|e| e.to_string())?;

    let mut rows = stmt.query([watch_result_id]).map_err(|e| e.to_string())?;

    let watch_result = rows.next().map_err(|e| e.to_string())?.ok_or("Résultat de veille non trouvé")?;

    let watch_result_obj = WatchResult {
        id: watch_result.get(0).ok(),
        watch_id: watch_result.get(1).unwrap_or_default(),
        title: watch_result.get(2).unwrap_or_default(),
        url: watch_result.get(3).unwrap_or_default(),
        content: watch_result.get(4).unwrap_or_default(),
        translated_content: watch_result.get(5).ok(),
        source_name: watch_result.get(6).unwrap_or_default(),
        published_at: watch_result.get(7).ok(),
        fetched_at: watch_result.get(8).ok(),
        is_processed: watch_result.get(9).unwrap_or(false),
        metadata: watch_result.get(10).ok(),
        entities: None,
    };

    let tor_client = state.tor_manager.lock().await;
    let api_key = state.settings.lock().await.api.mistral_key.clone();
    let mistral_client = MistralApiClient::new(tor_client.create_tor_client()?, api_key);

    let capitalization_manager = CapitalizationManager::new(
        Arc::clone(&state.db),
        tor_client.create_tor_client()?,
    ).with_api_client(mistral_client);

    // Traiter le résultat pour obtenir un ProcessedWatchResult
    let processor = super::watch::WatchProcessor::new(
        Arc::clone(&state.db),
        super::watch::ApiClient::new(
            tor_client.create_tor_client()?,
            state.settings.lock().await.api.mistral_key.clone(),
        ),
        Arc::new(super::watch::WatchManager::new(
            Arc::clone(&state.db),
            tor_client.create_tor_client()?,
            super::watch::ApiClient::new(
                tor_client.create_tor_client()?,
                state.settings.lock().await.api.mistral_key.clone(),
            ),
        )),
    );

    let processed = processor.process_result(watch_result_obj).await?;

    capitalization_manager.capitalize_watch_result(&processed, user_id).await
}


#[allow(dead_code)]
pub async fn capitalize_event_group(
    
    
        
        
            
        state: &crate::AppState,
    event_group: EventGroup,
    user_id: Option<i64>,
) -> Result<Vec<CapitalizedInfo>, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_key = state.settings.lock().await.api.mistral_key.clone();
    let mistral_client = MistralApiClient::new(tor_client.create_tor_client()?, api_key);

    let capitalization_manager = CapitalizationManager::new(
        Arc::clone(&state.db),
        tor_client.create_tor_client()?,
    ).with_api_client(mistral_client);

    capitalization_manager.capitalize_event_group(&event_group, user_id).await
}


#[allow(dead_code)]
pub async fn validate_capitalized_info(
    
    
        
        
            
        state: &crate::AppState,
    capitalized_info_id: i64,
    user_id: i64,
    is_valid: bool,
    comments: Option<String>,
) -> Result<(), String> {
    let tor_client = state.tor_manager.lock().await;
    let api_key = state.settings.lock().await.api.mistral_key.clone();
    let mistral_client = MistralApiClient::new(tor_client.create_tor_client()?, api_key);

    let capitalization_manager = CapitalizationManager::new(
        Arc::clone(&state.db),
        tor_client.create_tor_client()?,
    ).with_api_client(mistral_client);

    capitalization_manager.validate_capitalized_info(capitalized_info_id, user_id, is_valid, comments).await
}


#[allow(dead_code)]
pub async fn get_unvalidated_capitalized_infos(
    
    
        
        
            
        state: &crate::AppState,
    limit: Option<usize>,
) -> Result<Vec<CapitalizedInfo>, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_key = state.settings.lock().await.api.mistral_key.clone();
    let mistral_client = MistralApiClient::new(tor_client.create_tor_client()?, api_key);

    let capitalization_manager = CapitalizationManager::new(
        Arc::clone(&state.db),
        tor_client.create_tor_client()?,
    ).with_api_client(mistral_client);

    capitalization_manager.get_unvalidated_capitalized_infos(limit).await
}


#[allow(dead_code)]
pub async fn get_capitalized_infos_by_watch(
    
    
        
        
            
        state: &crate::AppState,
    watch_id: i64,
) -> Result<Vec<CapitalizedInfo>, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_key = state.settings.lock().await.api.mistral_key.clone();
    let mistral_client = MistralApiClient::new(tor_client.create_tor_client()?, api_key);

    let capitalization_manager = CapitalizationManager::new(
        Arc::clone(&state.db),
        tor_client.create_tor_client()?,
    ).with_api_client(mistral_client);

    capitalization_manager.get_capitalized_infos_by_watch(watch_id).await
}


#[allow(dead_code)]
pub async fn auto_capitalize_watch_result(
    
    
        
        
            
        state: &crate::AppState,
    watch_result_id: i64,
    user_id: Option<i64>,
) -> Result<CapitalizedInfo, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_key = state.settings.lock().await.api.mistral_key.clone();
    let mistral_client = MistralApiClient::new(tor_client.create_tor_client()?, api_key);

    let capitalization_manager = CapitalizationManager::new(
        Arc::clone(&state.db),
        tor_client.create_tor_client()?,
    ).with_api_client(mistral_client);

    let db = state.db.lock().await;
    
    let mut stmt = db.conn.prepare(
        "SELECT id, watch_id, title, url, content, translated_content, source_name, published_at, fetched_at, is_processed, metadata 
         FROM watch_results 
         WHERE id = ?"
    ).map_err(|e| e.to_string())?;

    let mut rows = stmt.query([watch_result_id]).map_err(|e| e.to_string())?;

    let watch_result = rows.next().map_err(|e| e.to_string())?.ok_or("Résultat de veille non trouvé")?;

    let watch_result_obj = WatchResult {
        id: watch_result.get(0).ok(),
        watch_id: watch_result.get(1).unwrap_or_default(),
        title: watch_result.get(2).unwrap_or_default(),
        url: watch_result.get(3).unwrap_or_default(),
        content: watch_result.get(4).unwrap_or_default(),
        translated_content: watch_result.get(5).ok(),
        source_name: watch_result.get(6).unwrap_or_default(),
        published_at: watch_result.get(7).ok(),
        fetched_at: watch_result.get(8).ok(),
        is_processed: watch_result.get(9).unwrap_or(false),
        metadata: watch_result.get(10).ok(),
        entities: None,
    };

    capitalization_manager.auto_capitalize(&watch_result_obj, user_id).await
}
