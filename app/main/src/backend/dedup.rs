use crate::AppState;
// Module de détection et gestion des doublons
// Gère l'identification et la fusion des entités en double

use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use log::{info, error, warn};
use serde::{Serialize, Deserialize};

use crate::backend::db::Database;
use osint_shared::models::EntityType;

/// Stratégie de détection de doublons
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeduplicationStrategy {
    /// Comparaison exacte des noms
    ExactMatch,
    /// Comparaison insensible à la casse
    CaseInsensitive,
    /// Comparaison avec distance de Levenshtein
    FuzzyMatch,
    /// Comparaison sémantique (via embeddings)
    SemanticMatch,
    /// Combinaison de toutes les stratégies
    Combined,
}

/// Résultat de la détection de doublons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateResult {
    pub entity_type: EntityType,
    pub entity_id: i64,
    pub entity_name: String,
    pub potential_duplicates: Vec<DuplicateCandidate>,
    pub is_duplicate: bool,
    pub best_match: Option<DuplicateCandidate>,
}

/// Candidat de doublon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCandidate {
    pub entity_id: i64,
    pub entity_name: String,
    pub similarity_score: f64,
    pub match_type: DeduplicationStrategy,
    pub metadata: serde_json::Value,
}

/// Gestionnaire de détection de doublons
pub struct DeduplicationManager {
    db: Arc<Mutex<Database>>,
}

impl DeduplicationManager {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    /// Vérifie si une entité est un doublon
    pub async fn check_for_duplicates(
        &self,
        entity_type: EntityType,
        name: &str,
        description: Option<&str>,
        strategy: DeduplicationStrategy,
    ) -> Result<DuplicateResult, String> {
        info!("Vérification des doublons pour {}: {}", entity_type.as_str(), name);

        let db = self.db.lock().await;
        
        // Récupérer les entités existantes du même type
        let existing_entities = db.get_entities_by_type(entity_type.clone())?;

        let mut potential_duplicates = Vec::new();

        for existing_entity in existing_entities {
            let existing_name = existing_entity.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let existing_description = existing_entity.get("description").and_then(|v| v.as_str());
            let existing_id = existing_entity.get("id").and_then(|v| v.as_i64()).unwrap_or(0);

            // Calculer le score de similarité
            let similarity_score = self.calculate_similarity(
                name,
                description,
                existing_name,
                existing_description,
                &strategy,
            );

            if similarity_score > 0.7 { // Seuil de similarité
                potential_duplicates.push(DuplicateCandidate {
                    entity_id: existing_id,
                    entity_name: existing_name.to_string(),
                    similarity_score,
                    match_type: strategy.clone(),
                    metadata: serde_json::json!({
                        "existing_entity": existing_entity
                    }),
                });
            }
        }

        // Trier par score de similarité (décroissant)
        potential_duplicates.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

        // Déterminer si c'est un doublon
        let is_duplicate = !potential_duplicates.is_empty();
        let best_match = potential_duplicates.first().cloned();

        Ok(DuplicateResult {
            entity_type,
            entity_id: 0, // À définir lors de la création
            entity_name: name.to_string(),
            potential_duplicates,
            is_duplicate,
            best_match,
        })
    }

    /// Calcule le score de similarité entre deux entités
    fn calculate_similarity(
        &self,
        name1: &str,
        desc1: Option<&str>,
        name2: &str,
        desc2: Option<&str>,
        strategy: &DeduplicationStrategy,
    ) -> f64 {
        match strategy {
            DeduplicationStrategy::ExactMatch => {
                if name1 == name2 {
                    1.0
                } else {
                    0.0
                }
            }
            DeduplicationStrategy::CaseInsensitive => {
                if name1.to_lowercase() == name2.to_lowercase() {
                    1.0
                } else {
                    0.0
                }
            }
            DeduplicationStrategy::FuzzyMatch => {
                // Utiliser la distance de Levenshtein
                let name_similarity = text_similarity(name1, name2);
                
                // Si des descriptions sont disponibles, les comparer aussi
                let desc_similarity = if let (Some(d1), Some(d2)) = (desc1, desc2) {
                    text_similarity(d1, d2)
                } else {
                    0.0
                };

                // Combinaison pondérée
                0.7 * name_similarity + 0.3 * desc_similarity
            }
            DeduplicationStrategy::SemanticMatch => {
                // En production, utiliser des embeddings pour la comparaison sémantique
                // Pour l'instant, on utilise une comparaison de mots-clés
                self.semantic_similarity(name1, desc1, name2, desc2)
            }
            DeduplicationStrategy::Combined => {
                // Combiner toutes les stratégies
                let exact = if name1 == name2 { 1.0 } else { 0.0 };
                let case_insensitive = if name1.to_lowercase() == name2.to_lowercase() { 1.0 } else { 0.0 };
                let fuzzy = text_similarity(name1, name2);
                let semantic = self.semantic_similarity(name1, desc1, name2, desc2);

                // Poids des différentes stratégies
                0.4 * exact + 0.2 * case_insensitive + 0.3 * fuzzy + 0.1 * semantic
            }
        }
    }

    /// Calcule la similarité sémantique (simplifiée)
    fn semantic_similarity(
        &self,
        name1: &str,
        desc1: Option<&str>,
        name2: &str,
        desc2: Option<&str>,
    ) -> f64 {
        // Extraire les mots-clés
        let keywords1 = extract_keywords(name1);
        let keywords2 = extract_keywords(name2);
        
        let desc_keywords1 = desc1.map(extract_keywords).unwrap_or_default();
        let desc_keywords2 = desc2.map(extract_keywords).unwrap_or_default();

        // Combiner les mots-clés
        let all_keywords1: Vec<&str> = keywords1.iter().chain(desc_keywords1.iter()).cloned().collect();
        let all_keywords2: Vec<&str> = keywords2.iter().chain(desc_keywords2.iter()).cloned().collect();

        // Calculer la similarité de Jaccard
        let set1: std::collections::HashSet<_> = all_keywords1.iter().collect();
        let set2: std::collections::HashSet<_> = all_keywords2.iter().collect();

        let intersection = set1.intersection(&set2).count() as f64;
        let union = set1.union(&set2).count() as f64;

        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }

    /// Fusionne deux entités en doublon
    pub async fn merge_entities(
        &self,
        source_entity_type: EntityType,
        source_entity_id: i64,
        target_entity_type: EntityType,
        target_entity_id: i64,
        keep_source: bool,
    ) -> Result<i64, String> {
        info!(
            "Fusion des entités: {}({}) -> {}({})",
            source_entity_type.as_str(), source_entity_id, target_entity_type.as_str(), target_entity_id
        );

        let db = self.db.lock().await;

        // Vérifier que les entités existent
        let source_entity = db.get_entity_by_id(source_entity_type.clone(), source_entity_id).map_err(|e| e.to_string())?
            .ok_or("Entité source non trouvée")?;
        
        let target_entity = db.get_entity_by_id(target_entity_type.clone(), target_entity_id).map_err(|e| e.to_string())?
            .ok_or("Entité cible non trouvée")?;

        // Déterminer quelle entité conserver
        let (entity_to_keep, entity_to_merge) = if keep_source {
            (source_entity, target_entity)
        } else {
            (target_entity, source_entity)
        };

        // Mettre à jour les références
        self.update_entity_references(
            &db,
            entity_to_merge.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
            entity_to_keep.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
            &entity_to_keep.get("entity_type").and_then(|v| v.as_str()).unwrap_or("unknown"),
        )?;

        // Supprimer l'entité fusionnée
        db.delete_entity(
            entity_to_merge.get("entity_type").and_then(|v| v.as_str()).unwrap_or("unknown"),
            entity_to_merge.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
        )?;

        // Créer un historique de fusion
        self.create_merge_history(
            &db,
            source_entity_type,
            source_entity_id,
            target_entity_type,
            target_entity_id,
            keep_source,
        )?;

        Ok(entity_to_keep.get("id").and_then(|v| v.as_i64()).unwrap_or(0))
    }

    /// Met à jour les références vers une entité
    fn update_entity_references(
        &self,
        db: &Database,
        old_entity_id: i64,
        new_entity_id: i64,
        entity_type: &str,
    ) -> Result<(), String> {
        // Mettre à jour les références dans les différentes tables
        // Ex: watch_results, capitalized_infos, etc.
        
        // Mettre à jour dans les résultats de veille
        db.conn.execute(
            "UPDATE watch_results SET entity_id = ? WHERE entity_id = ? AND entity_type = ?",
            (new_entity_id, old_entity_id, entity_type),
        ).map_err(|e| e.to_string())?;

        // Mettre à jour dans les informations capitalisées
        db.conn.execute(
            "UPDATE capitalized_infos SET entity_id = ? WHERE entity_id = ? AND entity_type = ?",
            (new_entity_id, old_entity_id, entity_type),
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Crée un historique de fusion
    fn create_merge_history(
        &self,
        db: &Database,
        source_type: EntityType,
        source_id: i64,
        target_type: EntityType,
        target_id: i64,
        keep_source: bool,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();

        db.conn.execute(
            "INSERT INTO entity_merge_history (source_type, source_id, target_type, target_id, keep_source, merged_at) 
             VALUES (?, ?, ?, ?, ?, ?)",
            (
                source_type.as_str(),
                source_id,
                target_type.as_str(),
                target_id,
                keep_source,
                now,
            ),
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Vérifie les doublons pour un lot d'entités
    pub async fn check_batch_for_duplicates(
        &self,
        entities: Vec<(EntityType, String, Option<String>)>,
        strategy: DeduplicationStrategy,
    ) -> Result<Vec<DuplicateResult>, String> {
        let mut results = Vec::new();

        for (entity_type, name, description) in entities {
            let result = self.check_for_duplicates(entity_type, &name, description.as_deref(), strategy.clone()).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Nettoie les doublons existants dans la base
    pub async fn cleanup_duplicates(
        &self,
        entity_type: EntityType,
        strategy: DeduplicationStrategy,
        auto_merge: bool,
    ) -> Result<Vec<DuplicateResult>, String> {
        info!("Nettoyage des doublons pour le type: {}", entity_type.as_str());

        let db = self.db.lock().await;
        let existing_entities = db.get_entities_by_type(entity_type.clone())?;

        let mut results = Vec::new();

        for entity in existing_entities {
            let entity_id = entity.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let entity_name = entity.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let entity_description = entity.get("description").and_then(|v| v.as_str());

            // Vérifier les doublons pour cette entité
            let mut duplicate_result = self.check_for_duplicates(
                entity_type.clone(),
                &entity_name,
                entity_description,
                strategy.clone(),
            ).await?;

            // Filtrer pour exclure l'entité elle-même
            duplicate_result.potential_duplicates.retain(|d| d.entity_id != entity_id);

            if auto_merge && !duplicate_result.potential_duplicates.is_empty() {
                // Fusionner automatiquement avec le meilleur candidat
                if let Some(best_match) = duplicate_result.best_match {
                    self.merge_entities(
                        entity_type.clone(),
                        entity_id,
                        entity_type.clone(),
                        best_match.entity_id,
                        true, // Conserver l'entité source
                    ).await?;
                }
            }

            results.push(duplicate_result);
        }

        Ok(results)
    }
}

/// Extrait les mots-clés d'un texte
fn extract_keywords(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|word| word.len() > 3) // Ignorer les mots trop courts
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
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

/// Commandes Tauri pour la détection de doublons


#[allow(dead_code)]
pub async fn check_duplicates(
    
    
        
        
            
        state: &crate::AppState,
    entity_type: String,
    name: String,
    description: Option<String>,
    strategy: Option<String>,
) -> Result<DuplicateResult, String> {
    let dedup_manager = DeduplicationManager::new(Arc::clone(&state.db));
    
    let entity_type_enum = EntityType::from_str(&entity_type)
        .ok_or_else(|| format!("Type d'entité inconnu: {}", entity_type))?;
    
    let strategy_enum = match strategy.as_deref() {
        Some("exact") => DeduplicationStrategy::ExactMatch,
        Some("case_insensitive") => DeduplicationStrategy::CaseInsensitive,
        Some("fuzzy") => DeduplicationStrategy::FuzzyMatch,
        Some("semantic") => DeduplicationStrategy::SemanticMatch,
        _ => DeduplicationStrategy::Combined,
    };

    dedup_manager.check_for_duplicates(entity_type_enum, &name, description.as_deref(), strategy_enum).await
}


#[allow(dead_code)]
pub async fn merge_entities(
    
    
        
        
            
        state: &crate::AppState,
    source_entity_type: String,
    source_entity_id: i64,
    target_entity_type: String,
    target_entity_id: i64,
    keep_source: bool,
) -> Result<i64, String> {
    let dedup_manager = DeduplicationManager::new(Arc::clone(&state.db));
    
    let source_type = EntityType::from_str(&source_entity_type)
        .ok_or_else(|| format!("Type d'entité source inconnu: {}", source_entity_type))?;
    let target_type = EntityType::from_str(&target_entity_type)
        .ok_or_else(|| format!("Type d'entité cible inconnu: {}", target_entity_type))?;

    dedup_manager.merge_entities(source_type, source_entity_id, target_type, target_entity_id, keep_source).await
}


#[allow(dead_code)]
pub async fn cleanup_duplicates(
    
    
        
        
            
        state: &crate::AppState,
    entity_type: String,
    strategy: Option<String>,
    auto_merge: bool,
) -> Result<Vec<DuplicateResult>, String> {
    let dedup_manager = DeduplicationManager::new(Arc::clone(&state.db));
    
    let entity_type_enum = EntityType::from_str(&entity_type)
        .ok_or_else(|| format!("Type d'entité inconnu: {}", entity_type))?;
    
    let strategy_enum = match strategy.as_deref() {
        Some("exact") => DeduplicationStrategy::ExactMatch,
        Some("case_insensitive") => DeduplicationStrategy::CaseInsensitive,
        Some("fuzzy") => DeduplicationStrategy::FuzzyMatch,
        Some("semantic") => DeduplicationStrategy::SemanticMatch,
        _ => DeduplicationStrategy::Combined,
    };

    dedup_manager.cleanup_duplicates(entity_type_enum, strategy_enum, auto_merge).await
}


#[allow(dead_code)]
pub async fn get_deduplication_strategies() -> Vec<String> {
    vec![
        "exact".to_string(),
        "case_insensitive".to_string(),
        "fuzzy".to_string(),
        "semantic".to_string(),
        "combined".to_string(),
    ]
}
