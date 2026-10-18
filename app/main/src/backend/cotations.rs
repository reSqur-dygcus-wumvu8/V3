//! Module de gestion des cotations OTAN
//! Gère le système de cotation A-F (source) et 1-6 (information)

use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use log::{info, error, warn};
use serde::{Serialize, Deserialize};

use crate::backend::db::Database;
use osint_shared::models::{RELIABILITY_CODES, CREDIBILITY_CODES};

/// Cotation OTAN complète (source + information)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OtanCotation {
    pub reliability: char, // A-F
    pub credibility: char, // 1-6
}

impl OtanCotation {
    /// Crée une nouvelle cotation
    pub fn new(reliability: char, credibility: char) -> Result<Self, String> {
        Self::validate(reliability, credibility)?;
        Ok(Self { reliability, credibility })
    }

    /// Valide une cotation
    pub fn validate(reliability: char, credibility: char) -> Result<(), String> {
        if !['A', 'B', 'C', 'D', 'E', 'F'].contains(&reliability) {
            return Err(format!("Code de fiabilité invalide: {}", reliability));
        }
        if !['1', '2', '3', '4', '5', '6'].contains(&credibility) {
            return Err(format!("Code de crédibilité invalide: {}", credibility));
        }
        Ok(())
    }

    /// Retourne la cotation au format "Xy" (ex: "A1", "B2")
    pub fn to_string(&self) -> String {
        format!("{}{}", self.reliability, self.credibility)
    }

    /// Parse une cotation depuis une chaîne (ex: "A1", "B2")
    pub fn from_str(cotation: &str) -> Result<Self, String> {
        if cotation.len() != 2 {
            return Err(format!("Format de cotation invalide: {}", cotation));
        }

        let reliability = cotation.chars().nth(0).unwrap();
        let credibility = cotation.chars().nth(1).unwrap();

        Self::validate(reliability, credibility)?;
        Ok(Self { reliability, credibility })
    }

    /// Retourne le libellé complet de la cotation
    pub fn get_label(&self) -> String {
        let reliability_label = RELIABILITY_CODES.iter()
            .find(|(code, _)| *code == self.reliability.to_string())
            .map(|(_, label)| *label)
            .unwrap_or("Inconnu");

        let credibility_label = CREDIBILITY_CODES.iter()
            .find(|(code, _)| *code == self.credibility.to_string())
            .map(|(_, label)| *label)
            .unwrap_or("Inconnu");

        format!("{} - {}", reliability_label, credibility_label)
    }

    /// Retourne le score numérique (1-6 pour la fiabilité, 1-6 pour la crédibilité)
    pub fn get_numeric_scores(&self) -> (u8, u8) {
        let reliability_score = match self.reliability {
            'A' => 1,
            'B' => 2,
            'C' => 3,
            'D' => 4,
            'E' => 5,
            'F' => 6,
            _ => 6,
        };

        let credibility_score = self.credibility.to_digit(10).unwrap_or(6) as u8;

        (reliability_score, credibility_score)
    }

    /// Compare deux cotations
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        let (self_reliability, self_credibility) = self.get_numeric_scores();
        let (other_reliability, other_credibility) = other.get_numeric_scores();

        // Comparer d'abord par fiabilité, puis par crédibilité
        self_reliability.cmp(&other_reliability)
            .then(self_credibility.cmp(&other_credibility))
    }
}

/// Gestionnaire des cotations
pub struct CotationManager {
    db: Arc<Mutex<Database>>,
}

impl CotationManager {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    /// Propose une cotation pour une source
    pub async fn propose_source_rating(
        &self,
        source_name: &str,
        source_url: Option<&str>,
        source_type: Option<&str>,
    ) -> Result<char, String> {
        info!("Proposition de cotation pour la source: {}", source_name);

        // Cotations par défaut pour les sources connues
        let source_ratings: std::collections::HashMap<&str, char> = [
            // Sources très fiables (A)
            ("BBC", 'A'),
            ("Le Monde", 'A'),
            ("AFP", 'A'),
            ("Reuters", 'A'),
            ("Associated Press", 'A'),
            ("AP News", 'A'),
            ("France 24", 'A'),
            ("RFI", 'A'),
            
            // Sources généralement fiables (B)
            ("Google Actualités", 'B'),
            ("Google News", 'B'),
            ("Qwant Actualités", 'B'),
            ("Qwant", 'B'),
            ("Wikipedia", 'B'),
            ("The Guardian", 'B'),
            ("New York Times", 'B'),
            ("Le Figaro", 'B'),
            ("Libération", 'B'),
            
            // Sources assez fiables (C)
            ("Yandex Actualités", 'C'),
            ("Yandex", 'C'),
            ("Twitter", 'C'),
            ("X", 'C'),
            ("Reddit", 'C'),
            ("VKontakte", 'C'),
            ("ВКонтакте", 'C'),
            
            // Sources peu fiables (D)
            ("Telegram", 'D'),
            ("Discord", 'D'),
            ("TikTok", 'D'),
            ("Facebook", 'D'),
            ("YouTube", 'D'),
            
            // Sources non évaluables (E/F)
            ("4chan", 'F'),
            ("8kun", 'F'),
        ]
        .iter()
        .cloned()
        .collect();

        // Rechercher une correspondance exacte
        if let Some(&rating) = source_ratings.get(source_name) {
            return Ok(rating);
        }

        // Rechercher dans les URLs
        if let Some(url) = source_url {
            for (name, &rating) in &source_ratings {
                if url.contains(name) {
                    return Ok(rating);
                }
            }
        }

        // Rechercher par type de source
        if let Some(source_type) = source_type {
            match source_type {
                "official_government" => Ok('A'),
                "news_agency" => Ok('B'),
                "social_media" => Ok('D'),
                "blog" => Ok('E'),
                "forum" => Ok('F'),
                _ => Ok('D'),
            }
        } else {
            // Cotation par défaut
            Ok('D')
        }
    }

    /// Propose une cotation pour une information
    pub async fn propose_info_rating(
        &self,
        source_rating: char,
        content: &str,
        entities: &[crate::backend::ner::DetectedEntity],
        metadata: Option<&serde_json::Value>,
    ) -> Result<char, String> {
        info!("Proposition de cotation pour l'information (source: {})", source_rating);

        // Logique de proposition basée sur plusieurs facteurs
        
        // 1. Fiabilité de la source
        let source_reliability_score = match source_rating {
            'A' => 1.0,
            'B' => 0.8,
            'C' => 0.6,
            'D' => 0.4,
            'E' => 0.2,
            'F' => 0.0,
            _ => 0.5,
        };

        // 2. Longueur et détail du contenu
        let content_score = if content.len() > 1000 {
            0.9
        } else if content.len() > 500 {
            0.7
        } else if content.len() > 200 {
            0.5
        } else {
            0.3
        };

        // 3. Nombre d'entités détectées (plus il y a d'entités, plus c'est crédible)
        let entity_score = if entities.len() > 10 {
            0.9
        } else if entities.len() > 5 {
            0.7
        } else if entities.len() > 0 {
            0.5
        } else {
            0.3
        };

        // 4. Présence de sources officielles dans le contenu
        let source_score = if content.contains("gouvernement") || 
                            content.contains("officiel") || 
                            content.contains("déclaration") {
            0.8
        } else {
            0.5
        };

        // 5. Métriques supplémentaires depuis les métadonnées
        let metadata_score = if let Some(metadata) = metadata {
            if metadata.get("engagement").and_then(|e| e.get("likes")).and_then(|l| l.as_u64()).unwrap_or(0) > 1000 {
                0.8
            } else {
                0.5
            }
        } else {
            0.5
        };

        // Calcul du score global (pondéré)
        let total_score = 0.4 * source_reliability_score +
                          0.2 * content_score +
                          0.2 * entity_score +
                          0.1 * source_score +
                          0.1 * metadata_score;

        // Conversion en cotation OTAN
        let credibility = if total_score >= 0.9 {
            '1' // Confirmée
        } else if total_score >= 0.7 {
            '2' // Probablement vraie
        } else if total_score >= 0.5 {
            '3' // Possiblement vraie
        } else if total_score >= 0.3 {
            '4' // Douteuse
        } else if total_score >= 0.1 {
            '5' // Improbable
        } else {
            '6' // Non évaluable
        };

        Ok(credibility)
    }

    /// Propose une cotation complète (source + information)
    pub async fn propose_cotation(
        &self,
        source_name: &str,
        source_url: Option<&str>,
        source_type: Option<&str>,
        content: &str,
        entities: &[crate::backend::ner::DetectedEntity],
        metadata: Option<&serde_json::Value>,
    ) -> Result<OtanCotation, String> {
        let source_rating = self.propose_source_rating(source_name, source_url, source_type).await?;
        let info_rating = self.propose_info_rating(source_rating, content, entities, metadata).await?;

        OtanCotation::new(source_rating, info_rating)
    }

    /// Sauvegarde une cotation dans la base
    pub async fn save_cotation(
        &self,
        entity_type: &str,
        entity_id: i64,
        cotation: &OtanCotation,
        rated_by: Option<i64>,
    ) -> Result<i64, String> {
        let db = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        let cotation_id = db.conn.execute(
            "INSERT INTO cotations (entity_type, entity_id, reliability_code, credibility_code, rated_by, rated_at) 
             VALUES (?, ?, ?, ?, ?, ?)",
            (
                entity_type,
                entity_id,
                cotation.reliability.to_string(),
                cotation.credibility.to_string(),
                rated_by.unwrap_or(0),
                now,
            ),
        ).map_err(|e| e.to_string())?;

        Ok(cotation_id as i64)
    }

    /// Récupère la cotation d'une entité
    pub async fn get_entity_cotation(
        &self,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Option<OtanCotation>, String> {
        let db = self.db.lock().unwrap();

        let mut stmt = db.conn.prepare(
            "SELECT reliability_code, credibility_code 
             FROM cotations 
             WHERE entity_type = ? AND entity_id = ? 
             ORDER BY rated_at DESC 
             LIMIT 1"
        ).map_err(|e| e.to_string())?;

        let mut rows = stmt.query([entity_type, entity_id])
            .map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let reliability: String = row.get(0).unwrap_or_default();
            let credibility: String = row.get(1).unwrap_or_default();

            if reliability.len() == 1 && credibility.len() == 1 {
                let reliability_char = reliability.chars().next().unwrap();
                let credibility_char = credibility.chars().next().unwrap();
                
                return Ok(Some(OtanCotation::new(reliability_char, credibility_char)?));
            }
        }

        Ok(None)
    }

    /// Récupère toutes les cotations d'une entité
    pub async fn get_entity_cotation_history(
        &self,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<(String, String, String)>, String> {
        let db = self.db.lock().unwrap();

        let mut stmt = db.conn.prepare(
            "SELECT reliability_code, credibility_code, rated_at 
             FROM cotations 
             WHERE entity_type = ? AND entity_id = ? 
             ORDER BY rated_at DESC"
        ).map_err(|e| e.to_string())?;

        let mut rows = stmt.query([entity_type, entity_id])
            .map_err(|e| e.to_string())?;

        let mut history = Vec::new();

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let reliability: String = row.get(0).unwrap_or_default();
            let credibility: String = row.get(1).unwrap_or_default();
            let rated_at: String = row.get(2).unwrap_or_default();

            history.push((reliability, credibility, rated_at));
        }

        Ok(history)
    }

    /// Met à jour la cotation d'une entité
    pub async fn update_entity_cotation(
        &self,
        entity_type: &str,
        entity_id: i64,
        cotation: &OtanCotation,
        rated_by: Option<i64>,
    ) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        db.conn.execute(
            "INSERT INTO cotations (entity_type, entity_id, reliability_code, credibility_code, rated_by, rated_at) 
             VALUES (?, ?, ?, ?, ?, ?)",
            (
                entity_type,
                entity_id,
                cotation.reliability.to_string(),
                cotation.credibility.to_string(),
                rated_by.unwrap_or(0),
                now,
            ),
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Calcule la cotation moyenne d'une entité
    pub async fn get_average_cotation(
        &self,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<OtanCotation, String> {
        let history = self.get_entity_cotation_history(entity_type, entity_id).await?;

        if history.is_empty() {
            // Retourner une cotation par défaut
            return OtanCotation::new('D', '4');
        }

        // Calculer la moyenne des scores
        let mut total_reliability = 0.0;
        let mut total_credibility = 0.0;

        for (reliability, credibility, _) in history {
            let reliability_score = match reliability.as_str() {
                "A" => 1.0,
                "B" => 2.0,
                "C" => 3.0,
                "D" => 4.0,
                "E" => 5.0,
                "F" => 6.0,
                _ => 4.0,
            };

            let credibility_score = match credibility.as_str() {
                "1" => 1.0,
                "2" => 2.0,
                "3" => 3.0,
                "4" => 4.0,
                "5" => 5.0,
                "6" => 6.0,
                _ => 4.0,
            };

            total_reliability += reliability_score;
            total_credibility += credibility_score;
        }

        let avg_reliability = total_reliability / history.len() as f64;
        let avg_credibility = total_credibility / history.len() as f64;

        // Arrondir et convertir en cotation
        let reliability = match avg_reliability.round() as u8 {
            1 => 'A',
            2 => 'B',
            3 => 'C',
            4 => 'D',
            5 => 'E',
            _ => 'F',
        };

        let credibility = match avg_credibility.round() as u8 {
            1 => '1',
            2 => '2',
            3 => '3',
            4 => '4',
            5 => '5',
            _ => '6',
        };

        OtanCotation::new(reliability, credibility)
    }
}

/// Commandes Tauri pour les cotations


#[tauri::command]
pub async fn propose_cotation(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    source_name: String,
    source_url: Option<String>,
    source_type: Option<String>,
    content: String,
    entities: Vec<crate::backend::ner::DetectedEntity>,
    metadata: Option<serde_json::Value>,
) -> Result<OtanCotation, String> {
    let cotation_manager = CotationManager::new(Arc::clone(&state.db));
    
    cotation_manager.propose_cotation(
        &source_name,
        source_url.as_deref(),
        source_type.as_deref(),
        &content,
        &entities,
        metadata.as_ref(),
    ).await
}


#[tauri::command]
pub async fn get_entity_cotation(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    entity_type: String,
    entity_id: i64,
) -> Result<Option<OtanCotation>, String> {
    let cotation_manager = CotationManager::new(Arc::clone(&state.db));
    
    cotation_manager.get_entity_cotation(&entity_type, entity_id).await
}


#[tauri::command]
pub async fn update_entity_cotation(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    entity_type: String,
    entity_id: i64,
    reliability: char,
    credibility: char,
    rated_by: Option<i64>,
) -> Result<(), String> {
    let cotation_manager = CotationManager::new(Arc::clone(&state.db));
    let cotation = OtanCotation::new(reliability, credibility)?;
    
    cotation_manager.update_entity_cotation(&entity_type, entity_id, &cotation, rated_by).await
}


#[tauri::command]
pub fn get_reliability_codes() -> Vec<(String, String)> {
    RELIABILITY_CODES.iter()
        .map(|(code, label)| (code.to_string(), label.to_string()))
        .collect()
}


#[tauri::command]
pub fn get_credibility_codes() -> Vec<(String, String)> {
    CREDIBILITY_CODES.iter()
        .map(|(code, label)| (code.to_string(), label.to_string()))
        .collect()
}


#[tauri::command]
pub fn validate_cotation(reliability: char, credibility: char) -> Result<(), String> {
    OtanCotation::validate(reliability, credibility)
}


#[tauri::command]
pub fn parse_cotation(cotation: String) -> Result<OtanCotation, String> {
    OtanCotation::from_str(&cotation)
}


#[tauri::command]
pub fn get_cotation_label(reliability: char, credibility: char) -> Result<String, String> {
    let cotation = OtanCotation::new(reliability, credibility)?;
    Ok(cotation.get_label())
}
