use osint_shared::models::EntityType;
use crate::AppState;
// Module GÉRER
// Gère la gestion des doublons, des paramètres et l'administration

use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use log::{info, error, warn};
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::backend::db::Database;
use osint_shared::models::AppSettings;
use super::{dedup::{DeduplicationManager, DeduplicationStrategy, DuplicateResult}, 
          entities::EntityManager, 
          cotations::CotationManager};

/// Élément de la file de revue des doublons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateReviewItem {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub entity_name: String,
    pub duplicate_entity_type: String,
    pub duplicate_entity_id: i64,
    pub duplicate_entity_name: String,
    pub similarity_score: f64,
    pub strategy: String,
    pub status: DuplicateReviewStatus,
    pub created_at: String,
    pub reviewed_by: Option<i64>,
    pub reviewed_at: Option<String>,
    pub action: Option<String>,
}

/// Statut de la revue des doublons
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DuplicateReviewStatus {
    Pending,
    Reviewed,
    Ignored,
    Merged,
}

impl DuplicateReviewStatus {
    pub async fn as_str(&self) -> &'static str {
        match self {
            DuplicateReviewStatus::Pending => "en_attente",
            DuplicateReviewStatus::Reviewed => "revue",
            DuplicateReviewStatus::Ignored => "ignore",
            DuplicateReviewStatus::Merged => "fusionne",
        }
    }

    pub async fn from_str(s: &str) -> Option<Self> {
        match s {
            "en_attente" | "pending" => Some(DuplicateReviewStatus::Pending),
            "revue" | "reviewed" => Some(DuplicateReviewStatus::Reviewed),
            "ignore" | "ignored" => Some(DuplicateReviewStatus::Ignored),
            "fusionne" | "merged" => Some(DuplicateReviewStatus::Merged),
            _ => None,
        }
    }
}

/// Action pour la gestion des doublons
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DuplicateAction {
    Merge,
    Ignore,
    KeepSeparate,
}

impl DuplicateAction {
    pub async fn as_str(&self) -> &'static str {
        match self {
            DuplicateAction::Merge => "fusionner",
            DuplicateAction::Ignore => "ignorer",
            DuplicateAction::KeepSeparate => "conserver_separe",
        }
    }

    pub async fn from_str(s: &str) -> Option<Self> {
        match s {
            "fusionner" | "merge" => Some(DuplicateAction::Merge),
            "ignorer" | "ignore" => Some(DuplicateAction::Ignore),
            "conserver_separe" | "keep_separate" => Some(DuplicateAction::KeepSeparate),
            _ => None,
        }
    }
}

/// Statistiques de la base de données
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_entities: usize,
    pub entities_by_type: std::collections::HashMap<String, usize>,
    pub total_watch_results: usize,
    pub total_capitalized_infos: usize,
    pub pending_duplicates: usize,
    pub db_size: u64, // en Mo
}

/// Gestionnaire du module GÉRER
pub struct GererManager {
    db: Arc<Mutex<Database>>,
}

impl GererManager {
    pub async fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    /// Détecte les doublons dans la base
    pub async fn detect_duplicates(
        &self,
        strategy: DeduplicationStrategy,
        auto_add_to_review: bool,
    ) -> Result<Vec<DuplicateResult>, String> {
        info!("Détection des doublons avec stratégie: {:?}", strategy);

        let dedup_manager = DeduplicationManager::new(Arc::clone(&self.db));
        let entity_types = [
            EntityType::Individual,
            EntityType::Organization,
            EntityType::Event,
            EntityType::Location,
            EntityType::Object,
        ];

        let mut all_duplicates = Vec::new();

        for entity_type in entity_types {
            let duplicates = dedup_manager.cleanup_duplicates(
                entity_type.clone(),
                strategy.clone(),
                false, // Ne pas fusionner automatiquement
            ).await?;

            all_duplicates.extend(duplicates);

            // Ajouter à la file de revue si demandé
            if auto_add_to_review {
                for duplicate in &duplicates {
                    if duplicate.is_duplicate {
                        self.add_to_duplicate_review_queue(duplicate, &strategy).await?;
                    }
                }
            }
        }

        Ok(all_duplicates)
    }

    /// Ajoute un doublon à la file de revue
    pub async fn add_to_duplicate_review_queue(
        &self,
        duplicate_result: &DuplicateResult,
        strategy: &DeduplicationStrategy,
    ) -> Result<(), String> {
        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        for candidate in &duplicate_result.potential_duplicates {
            db.conn.execute(
                "INSERT INTO duplicate_review_queue (entity_type, entity_id, entity_name, duplicate_entity_type, duplicate_entity_id, duplicate_entity_name, similarity_score, strategy, status, created_at) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    duplicate_result.entity_type.as_str(),
                    duplicate_result.entity_id,
                    duplicate_result.entity_name.clone(),
                    candidate.entity_type.as_str(),
                    candidate.entity_id,
                    candidate.entity_name.clone(),
                    candidate.similarity_score,
                    strategy.as_str(),
                    DuplicateReviewStatus::Pending.as_str(),
                    now.clone(),
                ),
            ).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Récupère la file de revue des doublons
    pub async fn get_duplicate_review_queue(
        &self,
        status: Option<DuplicateReviewStatus>,
        limit: Option<usize>,
    ) -> Result<Vec<DuplicateReviewItem>, String> {
        let db = self.db.lock().await;

        let status_filter = status.map(|s| format!(" AND status = '{}'", s.as_str())).unwrap_or_default();
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();

        let query = format!(
            "SELECT id, entity_type, entity_id, entity_name, duplicate_entity_type, duplicate_entity_id, duplicate_entity_name, similarity_score, strategy, status, created_at, reviewed_by, reviewed_at, action 
             FROM duplicate_review_queue 
             WHERE 1=1{} 
             ORDER BY created_at DESC{}",
            status_filter, limit_clause
        );

        let mut stmt = db.conn.prepare(&query).map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

        let mut items = Vec::new();

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            items.push(DuplicateReviewItem {
                id: row.get(0).unwrap_or_default(),
                entity_type: row.get(1).unwrap_or_default(),
                entity_id: row.get(2).unwrap_or_default(),
                entity_name: row.get(3).unwrap_or_default(),
                duplicate_entity_type: row.get(4).unwrap_or_default(),
                duplicate_entity_id: row.get(5).unwrap_or_default(),
                duplicate_entity_name: row.get(6).unwrap_or_default(),
                similarity_score: row.get(7).unwrap_or(0.0),
                strategy: row.get(8).unwrap_or_default(),
                status: DuplicateReviewStatus::from_str(&row.get::<_, String>(9).unwrap_or_default())
                    .unwrap_or(DuplicateReviewStatus::Pending),
                created_at: row.get(10).unwrap_or_default(),
                reviewed_by: row.get(11).ok(),
                reviewed_at: row.get(12).ok(),
                action: row.get(13).ok(),
            });
        }

        Ok(items)
    }

    /// Traite un élément de la file de revue
    pub async fn process_duplicate_review_item(
        &self,
        item_id: i64,
        user_id: i64,
        action: DuplicateAction,
        comments: Option<String>,
    ) -> Result<(), String> {
        info!("Traitement de l'élément de revue {} avec action: {:?}", item_id, action);

        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        // Récupérer l'élément
        let mut stmt = db.conn.prepare(
            "SELECT entity_type, entity_id, duplicate_entity_type, duplicate_entity_id 
             FROM duplicate_review_queue 
             WHERE id = ?"
        ).map_err(|e| e.to_string())?;

        let mut rows = stmt.query([item_id]).map_err(|e| e.to_string())?;

        let item = rows.next().map_err(|e| e.to_string())?.ok_or("Élément non trouvé")?;

        let entity_type = EntityType::from_str(&item.get::<_, String>(0).unwrap_or_default())
            .unwrap_or(EntityType::Source);
        let entity_id = item.get::<_, i64>(1).unwrap_or(0);
        let duplicate_entity_type = EntityType::from_str(&item.get::<_, String>(2).unwrap_or_default())
            .unwrap_or(EntityType::Source);
        let duplicate_entity_id = item.get::<_, i64>(3).unwrap_or(0);

        // Exécuter l'action
        match action {
            DuplicateAction::Merge => {
                // Fusionner les entités
                let dedup_manager = DeduplicationManager::new(Arc::clone(&self.db));
                dedup_manager.merge_entities(
                    entity_type.clone(),
                    entity_id,
                    duplicate_entity_type.clone(),
                    duplicate_entity_id,
                    true, // Conserver l'entité source
                ).await?;

                // Mettre à jour le statut
                db.conn.execute(
                    "UPDATE duplicate_review_queue SET status = ?, reviewed_by = ?, reviewed_at = ?, action = ? WHERE id = ?",
                    (
                        DuplicateReviewStatus::Merged.as_str(),
                        user_id,
                        now,
                        action.as_str(),
                        item_id,
                    ),
                ).map_err(|e| e.to_string())?;
            }
            DuplicateAction::Ignore => {
                // Marquer comme ignoré
                db.conn.execute(
                    "UPDATE duplicate_review_queue SET status = ?, reviewed_by = ?, reviewed_at = ?, action = ? WHERE id = ?",
                    (
                        DuplicateReviewStatus::Ignored.as_str(),
                        user_id,
                        now,
                        action.as_str(),
                        item_id,
                    ),
                ).map_err(|e| e.to_string())?;
            }
            DuplicateAction::KeepSeparate => {
                // Marquer comme revu mais conserver séparé
                db.conn.execute(
                    "UPDATE duplicate_review_queue SET status = ?, reviewed_by = ?, reviewed_at = ?, action = ? WHERE id = ?",
                    (
                        DuplicateReviewStatus::Reviewed.as_str(),
                        user_id,
                        now,
                        action.as_str(),
                        item_id,
                    ),
                ).map_err(|e| e.to_string())?;
            }
        }

        // Sauvegarder les commentaires
        if let Some(comments) = comments {
            db.conn.execute(
                "UPDATE duplicate_review_queue SET comments = ? WHERE id = ?",
                (comments, item_id),
            ).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Nettoie les doublons automatiquement
    pub async fn auto_clean_duplicates(
        &self,
        strategy: DeduplicationStrategy,
        min_similarity: f64,
        user_id: Option<i64>,
    ) -> Result<Vec<DuplicateReviewItem>, String> {
        info!("Nettoyage automatique des doublons (similarité >= {})", min_similarity);

        let dedup_manager = DeduplicationManager::new(Arc::clone(&self.db));
        let entity_types = [
            EntityType::Individual,
            EntityType::Organization,
            EntityType::Event,
            EntityType::Location,
            EntityType::Object,
        ];

        let mut processed_items = Vec::new();

        for entity_type in entity_types {
            let duplicates = dedup_manager.cleanup_duplicates(
                entity_type.clone(),
                strategy.clone(),
                true, // Fusionner automatiquement
            ).await?;

            for duplicate in duplicates {
                if duplicate.is_duplicate {
                    if let Some(best_match) = duplicate.best_match {
                        if best_match.similarity_score >= min_similarity {
                            // Fusionner automatiquement
                            dedup_manager.merge_entities(
                                entity_type.clone(),
                                duplicate.entity_id,
                                EntityType::from_str(&best_match.entity_type.as_str()).unwrap_or(EntityType::Source),
                                best_match.entity_id,
                                true,
                            ).await?;

                            // Créer un élément de revue pour l'historique
                            let item = DuplicateReviewItem {
                                id: 0, // À définir
                                entity_type: entity_type.as_str().to_string(),
                                entity_id: duplicate.entity_id,
                                entity_name: duplicate.entity_name.clone(),
                                duplicate_entity_type: best_match.entity_type.as_str().to_string(),
                                duplicate_entity_id: best_match.entity_id,
                                duplicate_entity_name: best_match.entity_name.clone(),
                                similarity_score: best_match.similarity_score,
                                strategy: strategy.as_str().to_string(),
                                status: DuplicateReviewStatus::Merged,
                                created_at: chrono::Utc::now().to_rfc3339(),
                                reviewed_by: user_id,
                                reviewed_at: Some(chrono::Utc::now().to_rfc3339()),
                                action: Some(DuplicateAction::Merge.as_str().to_string()),
                            };

                            processed_items.push(item);
                        }
                    }
                }
            }
        }

        Ok(processed_items)
    }

    /// Récupère les statistiques de la base
    pub async fn get_database_stats(&self) -> Result<DatabaseStats, String> {
        let db = self.db.lock().await;

        // Compter les entités par type
        let entity_types = ["individuals", "organizations", "events", "locations", "objects", "sources"];
        let mut entities_by_type = std::collections::HashMap::new();
        let mut total_entities = 0;

        for entity_type in entity_types {
            let count: i64 = db.conn.query_row(
                &format!("SELECT COUNT(*) FROM {}", entity_type),
                [],
                |row| row.get(0),
            ).map_err(|e| e.to_string())?;

            entities_by_type.insert(entity_type.to_string(), count as usize);
            total_entities += count as usize;
        }

        // Compter les résultats de veille
        let watch_results_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM watch_results",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        // Compter les informations capitalisées
        let capitalized_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM capitalized_infos",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        // Compter les doublons en attente
        let pending_duplicates: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM duplicate_review_queue WHERE status = 'en_attente'",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        // Calculer la taille de la base (approximative)
        // En production, utiliser la taille du fichier de la base
        let db_size = 0; // À implémenter

        Ok(DatabaseStats {
            total_entities,
            entities_by_type,
            total_watch_results: watch_results_count as usize,
            total_capitalized_infos: capitalized_count as usize,
            pending_duplicates: pending_duplicates as usize,
            db_size,
        })
    }

    /// Sauvegarde les paramètres de l'application
    pub async fn save_settings(&self, settings: AppSettings) -> Result<(), String> {
        settings.save()
    }

    /// Récupère les paramètres de l'application
    pub async fn get_settings(&self) -> Result<AppSettings, String> {
        AppSettings::load()
    }

    /// Sauvegarde une sauvegarde de la base
    pub async fn backup_database(&self, backup_path: &str) -> Result<(), String> {
        let db = self.db.lock().await;

        // Copier le fichier de la base
        let db_path = db.get_path()?;
        
        std::fs::copy(&db_path, backup_path)
            .map_err(|e| format!("Erreur lors de la sauvegarde: {}", e))?;

        info!("Sauvegarde de la base créée: {}", backup_path);

        Ok(())
    }

    /// Restaure une sauvegarde de la base
    pub async fn restore_database(&self, backup_path: &str) -> Result<(), String> {
        let db = self.db.lock().await;

        // Fermer la connexion actuelle
        // En production, il faudrait redémarrer l'application
        
        // Copier le fichier de sauvegarde
        let db_path = db.get_path()?;
        
        std::fs::copy(backup_path, &db_path)
            .map_err(|e| format!("Erreur lors de la restauration: {}", e))?;

        info!("Base restaurée depuis: {}", backup_path);

        Ok(())
    }

    /// Optimise la base de données
    pub async fn optimize_database(&self) -> Result<(), String> {
        let db = self.db.lock().await;

        // Exécuter VACUUM pour optimiser la base
        db.conn.execute("VACUUM", []).map_err(|e| e.to_string())?;

        // Exécuter REINDEX
        db.conn.execute("REINDEX", []).map_err(|e| e.to_string())?;

        info!("Base de données optimisée");

        Ok(())
    }

    /// Vérifie l'intégrité de la base
    pub async fn check_database_integrity(&self) -> Result<Vec<String>, String> {
        let db = self.db.lock().await;
        let mut errors = Vec::new();

        // Vérifier les tables
        let tables = ["individuals", "organizations", "events", "locations", "objects", "sources", 
                      "watch_results", "capitalized_infos", "entity_links", "cotations"];

        for table in tables {
            let result: Result<i64, _> = db.conn.query_row(
                &format!("SELECT COUNT(*) FROM {}", table),
                [],
                |row| row.get(0),
            );

            if let Err(e) = result {
                errors.push(format!("Table {}: {}", table, e));
            }
        }

        // Vérifier les liens d'entités
        let orphans: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM entity_links el 
             WHERE NOT EXISTS (SELECT 1 FROM individuals i WHERE i.id = el.source_id AND el.source_type = 'individual') 
                AND NOT EXISTS (SELECT 1 FROM organizations o WHERE o.id = el.source_id AND el.source_type = 'organization')
                AND NOT EXISTS (SELECT 1 FROM events e WHERE e.id = el.source_id AND el.source_type = 'event')
                AND NOT EXISTS (SELECT 1 FROM locations l WHERE l.id = el.source_id AND el.source_type = 'location')
                AND NOT EXISTS (SELECT 1 FROM objects o WHERE o.id = el.source_id AND el.source_type = 'object')
                AND NOT EXISTS (SELECT 1 FROM sources s WHERE s.id = el.source_id AND el.source_type = 'source')",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        if orphans > 0 {
            errors.push(format!("{} liens orphelins trouvés", orphans));
        }

        if errors.is_empty() {
            Ok(vec!["Base de données intègre".to_string()])
        } else {
            Ok(errors)
        }
    }

    /// Exporte les données au format JSON
    pub async fn export_data_json(&self, entity_type: Option<String>) -> Result<String, String> {
        let db = self.db.lock().await;

        if let Some(entity_type) = entity_type {
            // Exporter un type d'entité spécifique
            let query = match entity_type.as_str() {
                "individuals" => "SELECT * FROM individuals",
                "organizations" => "SELECT * FROM organizations",
                "events" => "SELECT * FROM events",
                "locations" => "SELECT * FROM locations",
                "objects" => "SELECT * FROM objects",
                "sources" => "SELECT * FROM sources",
                _ => return Err(format!("Type d'entité inconnu: {}", entity_type)),
            };

            let mut stmt = db.conn.prepare(query).map_err(|e| e.to_string())?;
            let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

            let mut results = Vec::new();

            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                let mut entity = serde_json::Map::new();
                
                for (idx, column) in row.as_ref().unwrap().columns().iter().enumerate() {
                    let value: serde_json::Value = row.get_ref_unwrap(idx).unwrap().into();
                    entity.insert(column.name().to_string(), value);
                }

                results.push(serde_json::Value::Object(entity));
            }

            Ok(serde_json::to_string_pretty(&results).unwrap_or_default())
        } else {
            // Exporter toutes les données
            let mut all_data = serde_json::Map::new();

            for entity_type in ["individuals", "organizations", "events", "locations", "objects", "sources"] {
                let query = format!("SELECT * FROM {}", entity_type);

                let mut stmt = db.conn.prepare(&query).map_err(|e| e.to_string())?;
                let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

                let mut results = Vec::new();

                while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                    let mut entity = serde_json::Map::new();
                    
                    for (idx, column) in row.as_ref().unwrap().columns().iter().enumerate() {
                        let value: serde_json::Value = row.get_ref_unwrap(idx).unwrap().into();
                        entity.insert(column.name().to_string(), value);
                    }

                    results.push(serde_json::Value::Object(entity));
                }

                all_data.insert(entity_type.to_string(), serde_json::Value::Array(results));
            }

            Ok(serde_json::to_string_pretty(&all_data).unwrap_or_default())
        }
    }

    /// Importe des données depuis JSON
    pub async fn import_data_json(&self, json_data: &str) -> Result<(), String> {
        let data: serde_json::Value = serde_json::from_str(json_data)
            .map_err(|e| format!("JSON invalide: {}", e))?;

        let db = self.db.lock().await;
        let tx = db.conn.transaction().map_err(|e| e.to_string())?;

        if let Some(obj) = data.as_object() {
            for (entity_type, entities) in obj {
                if let Some(array) = entities.as_array() {
                    for entity in array {
                        self.import_entity(&tx, entity_type, entity)?;
                    }
                }
            }
        }

        tx.commit().map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Importe une entité
    fn import_entity(
        &self,
        tx: &rusqlite::Transaction,
        entity_type: &str,
        entity: &serde_json::Value,
    ) -> Result<(), String> {
        let table_name = match entity_type {
            "individuals" => "individuals",
            "organizations" => "organizations",
            "events" => "events",
            "locations" => "locations",
            "objects" => "objects",
            "sources" => "sources",
            _ => return Err(format!("Type d'entité inconnu: {}", entity_type)),
        };

        let obj = entity.as_object().ok_or("Entité non valide")?;

        // Construire la requête d'insertion
        let columns: Vec<String> = obj.keys().cloned().collect();
        let placeholders: Vec<String> = vec!["?"; columns.len()];

        let query = format!(
            "INSERT OR IGNORE INTO {} ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            placeholders.join(", ")
        );

        let values: Vec<Box<dyn rusqlite::ToSql>> = columns.iter()
            .map(|col| {
                let value = obj.get(col).unwrap();
                match value {
                    serde_json::Value::Null => Box::new(rusqlite::types::Null) as Box<dyn rusqlite::ToSql>,
                    serde_json::Value::Bool(b) => Box::new(*b) as Box<dyn rusqlite::ToSql>,
                    serde_json::Value::Number(n) => {
                        if n.is_f64() {
                            Box::new(n.as_f64().unwrap()) as Box<dyn rusqlite::ToSql>
                        } else {
                            Box::new(n.as_i64().unwrap()) as Box<dyn rusqlite::ToSql>
                        }
                    }
                    serde_json::Value::String(s) => Box::new(s.clone()) as Box<dyn rusqlite::ToSql>,
                    serde_json::Value::Array(_) => Box::new(s.to_string()) as Box<dyn rusqlite::ToSql>,
                    serde_json::Value::Object(_) => Box::new(s.to_string()) as Box<dyn rusqlite::ToSql>,
                }
            })
            .collect();

        tx.execute(&query, values.as_slice())
            .map_err(|e| format!("Erreur lors de l'import de l'entité {}: {}", entity_type, e))?;

        Ok(())
    }
}

/// Commandes Tauri pour le module GÉRER


#[allow(dead_code)]
pub async fn detect_duplicates(
    
    
        
        
            
        state: &crate::AppState,
    strategy: String,
    auto_add_to_review: bool,
) -> Result<Vec<DuplicateResult>, String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    let strategy_enum = DeduplicationStrategy::from_str(&strategy)
        .ok_or_else(|| format!("Stratégie de détection inconnue: {}", strategy))?;
    
    gerer_manager.detect_duplicates(strategy_enum, auto_add_to_review).await
}


#[allow(dead_code)]
pub async fn get_duplicate_review_queue(
    
    
        
        
            
        state: &crate::AppState,
    status: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<DuplicateReviewItem>, String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    let status_enum = status.and_then(|s| DuplicateReviewStatus::from_str(&s).ok());
    
    gerer_manager.get_duplicate_review_queue(status_enum, limit).await
}


#[allow(dead_code)]
pub async fn process_duplicate_review_item(
    
    
        
        
            
        state: &crate::AppState,
    item_id: i64,
    user_id: i64,
    action: String,
    comments: Option<String>,
) -> Result<(), String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    let action_enum = DuplicateAction::from_str(&action)
        .ok_or_else(|| format!("Action inconnue: {}", action))?;
    
    gerer_manager.process_duplicate_review_item(item_id, user_id, action_enum, comments).await
}


#[allow(dead_code)]
pub async fn auto_clean_duplicates(
    
    
        
        
            
        state: &crate::AppState,
    strategy: String,
    min_similarity: f64,
    user_id: Option<i64>,
) -> Result<Vec<DuplicateReviewItem>, String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    let strategy_enum = DeduplicationStrategy::from_str(&strategy)
        .ok_or_else(|| format!("Stratégie de détection inconnue: {}", strategy))?;
    
    gerer_manager.auto_clean_duplicates(strategy_enum, min_similarity, user_id).await
}


#[allow(dead_code)]
pub async fn get_database_stats(
    
    
        
        
            
        state: &crate::AppState,
) -> Result<DatabaseStats, String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    gerer_manager.get_database_stats().await
}


#[allow(dead_code)]
pub async fn backup_database(
    
    
        
        
            
        state: &crate::AppState,
    backup_path: String,
) -> Result<(), String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    gerer_manager.backup_database(&backup_path).await
}


#[allow(dead_code)]
pub async fn restore_database(
    
    
        
        
            
        state: &crate::AppState,
    backup_path: String,
) -> Result<(), String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    gerer_manager.restore_database(&backup_path).await
}


#[allow(dead_code)]
pub async fn optimize_database(
    
    
        
        
            
        state: &crate::AppState,
) -> Result<(), String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    gerer_manager.optimize_database().await
}


#[allow(dead_code)]
pub async fn check_database_integrity(
    
    
        
        
            
        state: &crate::AppState,
) -> Result<Vec<String>, String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    gerer_manager.check_database_integrity().await
}


#[allow(dead_code)]
pub async fn export_data_json(
    
    
        
        
            
        state: &crate::AppState,
    entity_type: Option<String>,
) -> Result<String, String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    gerer_manager.export_data_json(entity_type).await
}


#[allow(dead_code)]
pub async fn import_data_json(
    
    
        
        
            
        state: &crate::AppState,
    json_data: String,
) -> Result<(), String> {
    let gerer_manager = GererManager::new(Arc::clone(&state.db));
    gerer_manager.import_data_json(&json_data).await
}


#[allow(dead_code)]
pub async fn get_duplicate_review_statuses() -> Vec<String> {
    vec![
        DuplicateReviewStatus::Pending.as_str().to_string(),
        DuplicateReviewStatus::Reviewed.as_str().to_string(),
        DuplicateReviewStatus::Ignored.as_str().to_string(),
        DuplicateReviewStatus::Merged.as_str().to_string(),
    ]
}


#[allow(dead_code)]
pub async fn get_duplicate_actions() -> Vec<String> {
    vec![
        DuplicateAction::Merge.as_str().to_string(),
        DuplicateAction::Ignore.as_str().to_string(),
        DuplicateAction::KeepSeparate.as_str().to_string(),
    ]
}
