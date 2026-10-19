use crate::AppState;
// Module de gestion des entités de la base de connaissance
// Gère la création, modification, recherche et suppression des entités

use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use log::{info, error, warn};
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::backend::db::Database;
use osint_shared::models::EntityType;
use super::{ner::DetectedEntity, cotations::OtanCotation, dedup::DeduplicationManager};

/// Type d'individu
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndividualType {
    Real,
    Fictive,
    Numerique,
}

impl IndividualType {
    pub async fn as_str(&self) -> &'static str {
        match self {
            IndividualType::Real => "réel",
            IndividualType::Fictive => "fictif",
            IndividualType::Numerique => "numérique",
        }
    }

    pub async fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "réel" | "real" | "réelle" => Some(IndividualType::Real),
            "fictif" | "fictive" | "fiction" => Some(IndividualType::Fictive),
            "numérique" | "numerique" | "digital" | "en ligne" => Some(IndividualType::Numerique),
            _ => None,
        }
    }
}

/// Type de lieu
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocationType {
    Real,
    Fictive,
    Numerique,
}

impl LocationType {
    pub async fn as_str(&self) -> &'static str {
        match self {
            LocationType::Real => "réel",
            LocationType::Fictive => "fictif",
            LocationType::Numerique => "numérique",
        }
    }

    pub async fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "réel" | "real" | "réelle" => Some(LocationType::Real),
            "fictif" | "fictive" | "fiction" => Some(LocationType::Fictive),
            "numérique" | "numerique" | "digital" | "en ligne" | "ip" => Some(LocationType::Numerique),
            _ => None,
        }
    }
}

/// Type d'organisation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrganizationType {
    Gouvernement,
    Entreprise,
    ONG,
    Media,
    Militaire,
    Criminel,
    Religieux,
    Educatif,
    Autre,
}

impl OrganizationType {
    pub async fn as_str(&self) -> &'static str {
        match self {
            OrganizationType::Gouvernement => "gouvernement",
            OrganizationType::Entreprise => "entreprise",
            OrganizationType::ONG => "ONG",
            OrganizationType::Media => "média",
            OrganizationType::Militaire => "militaire",
            OrganizationType::Criminel => "criminel",
            OrganizationType::Religieux => "religieux",
            OrganizationType::Educatif => "éducatif",
            OrganizationType::Autre => "autre",
        }
    }

    pub async fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "gouvernement" | "government" | "état" | "state" => Some(OrganizationType::Gouvernement),
            "entreprise" | "company" | "société" | "corporation" => Some(OrganizationType::Entreprise),
            "ong" | "ngo" | "organisation non gouvernementale" => Some(OrganizationType::ONG),
            "média" | "media" | "presse" | "journal" => Some(OrganizationType::Media),
            "militaire" | "army" | "armée" | "forces armées" => Some(OrganizationType::Militaire),
            "criminel" | "criminal" | "mafia" | "gang" => Some(OrganizationType::Criminel),
            "religieux" | "religion" | "église" | "mosquée" | "temple" => Some(OrganizationType::Religieux),
            "éducatif" | "education" | "école" | "université" => Some(OrganizationType::Educatif),
            _ => Some(OrganizationType::Autre),
        }
    }
}

/// Type d'objet
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ObjectType {
    Vehicule,
    Arme,
    Document,
    Equipement,
    Bâtiment,
    Autre,
}

impl ObjectType {
    pub async fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Vehicule => "véhicule",
            ObjectType::Arme => "arme",
            ObjectType::Document => "document",
            ObjectType::Equipement => "équipement",
            ObjectType::Bâtiment => "bâtiment",
            ObjectType::Autre => "autre",
        }
    }

    pub async fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "véhicule" | "vehicle" | "voiture" | "camion" | "avion" => Some(ObjectType::Vehicule),
            "arme" | "weapon" | "pistolet" | "fusil" | "missile" => Some(ObjectType::Arme),
            "document" | "doc" | "fichier" | "file" => Some(ObjectType::Document),
            "équipement" | "equipment" | "matériel" => Some(ObjectType::Equipement),
            "bâtiment" | "building" | "immeuble" | "maison" => Some(ObjectType::Bâtiment),
            _ => Some(ObjectType::Autre),
        }
    }
}

/// Entité générique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: Option<i64>,
    pub entity_type: EntityType,
    pub name: String,
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub cotation: Option<OtanCotation>,
    pub is_active: bool,
}

/// Individu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Individual {
    pub id: Option<i64>,
    pub entity_type: EntityType,
    pub individual_type: IndividualType,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub full_name: String,
    pub biography: Option<String>,
    pub comments: Option<String>,
    pub date_of_birth: Option<String>,
    pub place_of_birth: Option<String>,
    pub nationality: Option<String>,
    pub linked_real_individual_id: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub cotation: Option<OtanCotation>,
    pub is_active: bool,
}

/// Organisation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Option<i64>,
    pub entity_type: EntityType,
    pub name: String,
    pub organization_type: OrganizationType,
    pub summary: Option<String>,
    pub comments: Option<String>,
    pub founded_date: Option<String>,
    pub headquarters: Option<String>,
    pub website: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub cotation: Option<OtanCotation>,
    pub is_active: bool,
}

/// Événement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<i64>,
    pub entity_type: EntityType,
    pub name: String,
    pub date_time: Option<String>,
    pub end_date_time: Option<String>,
    pub summary: Option<String>,
    pub comments: Option<String>,
    pub location: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub cotation: Option<OtanCotation>,
    pub is_active: bool,
}

/// Lieu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: Option<i64>,
    pub entity_type: EntityType,
    pub location_type: LocationType,
    pub name: String,
    pub address: Option<String>,
    pub coordinates: Option<String>,
    pub summary: Option<String>,
    pub comments: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub cotation: Option<OtanCotation>,
    pub is_active: bool,
}

/// Objet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub id: Option<i64>,
    pub entity_type: EntityType,
    pub object_type: ObjectType,
    pub name: String,
    pub details: Option<String>,
    pub comments: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub cotation: Option<OtanCotation>,
    pub is_active: bool,
}

/// Source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: Option<i64>,
    pub entity_type: EntityType,
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub reliability: Option<char>,
    pub comments: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub is_active: bool,
}

/// Gestionnaire des entités
pub struct EntityManager {
    db: Arc<Mutex<Database>>,
    dedup_manager: DeduplicationManager,
}

impl EntityManager {
    pub async fn new(db: Arc<Mutex<Database>>) -> Self {
        Self {
            db: Arc::clone(&db),
            dedup_manager: DeduplicationManager::new(Arc::clone(&db)).await,
        }
    }

    /// Crée une entité générique
    pub async fn create_entity(
        &self,
        entity_type: EntityType,
        name: String,
        description: Option<String>,
        cotation: Option<OtanCotation>,
    ) -> Result<i64, String> {
        info!("Création de l'entité: {} ({})", name, entity_type.as_str());

        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        // Vérifier les doublons
        let duplicate_check = self.dedup_manager.check_for_duplicates(
            entity_type.clone(),
            &name,
            description.as_deref(),
            super::dedup::DeduplicationStrategy::Combined,
        ).await?;

        if duplicate_check.is_duplicate {
            if let Some(best_match) = duplicate_check.best_match {
                return Err(format!(
                    "Doublon détecté: {} (ID: {})",
                    best_match.entity_name, best_match.entity_id
                ));
            }
        }

        let entity_id = db.conn.execute(
            "INSERT INTO entities (entity_type, name, description, created_at, updated_at, cotation, is_active) 
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                entity_type.as_str(),
                name,
                description.unwrap_or_default(),
                now.clone(),
                now.clone(),
                cotation.map(|c| c.to_string()).unwrap_or_default(),
                true,
            ),
        ).map_err(|e| e.to_string())?;

        // Sauvegarder la cotation
        if let Some(cotation) = cotation {
            db.conn.execute(
                "INSERT INTO cotations (entity_type, entity_id, reliability_code, credibility_code, rated_at) 
                 VALUES (?, ?, ?, ?, ?)",
                (
                    entity_type.as_str(),
                    entity_id as i64,
                    cotation.reliability.to_string(),
                    cotation.credibility.to_string(),
                    now,
                ),
            ).map_err(|e| e.to_string())?;
        }

        Ok(entity_id as i64)
    }

    /// Crée un individu
    pub async fn create_individual(
        &self,
        individual_type: IndividualType,
        first_name: Option<String>,
        last_name: Option<String>,
        full_name: String,
        biography: Option<String>,
        comments: Option<String>,
        date_of_birth: Option<String>,
        place_of_birth: Option<String>,
        nationality: Option<String>,
        linked_real_individual_id: Option<i64>,
        cotation: Option<OtanCotation>,
    ) -> Result<i64, String> {
        info!("Création de l'individu: {}", full_name);

        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        // Si c'est un individu fictif ou numérique, vérifier qu'il est lié à un individu réel
        if individual_type != IndividualType::Real && linked_real_individual_id.is_none() {
            return Err("Un individu fictif ou numérique doit être lié à un individu réel".to_string());
        }

        let entity_id = db.conn.execute(
            "INSERT INTO individuals (entity_type, individual_type, first_name, last_name, full_name, biography, comments, date_of_birth, place_of_birth, nationality, linked_real_individual_id, created_at, updated_at, cotation, is_active) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                EntityType::Individual.as_str(),
                individual_type.as_str(),
                first_name,
                last_name,
                full_name,
                biography,
                comments,
                date_of_birth,
                place_of_birth,
                nationality,
                linked_real_individual_id,
                now.clone(),
                now.clone(),
                cotation.map(|c| c.to_string()).unwrap_or_default(),
                true,
            ),
        ).map_err(|e| e.to_string())?;

        // Sauvegarder la cotation
        if let Some(cotation) = cotation {
            db.conn.execute(
                "INSERT INTO cotations (entity_type, entity_id, reliability_code, credibility_code, rated_at) 
                 VALUES (?, ?, ?, ?, ?)",
                (
                    EntityType::Individual.as_str(),
                    entity_id as i64,
                    cotation.reliability.to_string(),
                    cotation.credibility.to_string(),
                    now,
                ),
            ).map_err(|e| e.to_string())?;
        }

        Ok(entity_id as i64)
    }

    /// Crée une organisation
    pub async fn create_organization(
        &self,
        name: String,
        organization_type: OrganizationType,
        summary: Option<String>,
        comments: Option<String>,
        founded_date: Option<String>,
        headquarters: Option<String>,
        website: Option<String>,
        cotation: Option<OtanCotation>,
    ) -> Result<i64, String> {
        info!("Création de l'organisation: {}", name);

        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        let entity_id = db.conn.execute(
            "INSERT INTO organizations (entity_type, name, organization_type, summary, comments, founded_date, headquarters, website, created_at, updated_at, cotation, is_active) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                EntityType::Organization.as_str(),
                name,
                organization_type.as_str(),
                summary,
                comments,
                founded_date,
                headquarters,
                website,
                now.clone(),
                now.clone(),
                cotation.map(|c| c.to_string()).unwrap_or_default(),
                true,
            ),
        ).map_err(|e| e.to_string())?;

        // Sauvegarder la cotation
        if let Some(cotation) = cotation {
            db.conn.execute(
                "INSERT INTO cotations (entity_type, entity_id, reliability_code, credibility_code, rated_at) 
                 VALUES (?, ?, ?, ?, ?)",
                (
                    EntityType::Organization.as_str(),
                    entity_id as i64,
                    cotation.reliability.to_string(),
                    cotation.credibility.to_string(),
                    now,
                ),
            ).map_err(|e| e.to_string())?;
        }

        Ok(entity_id as i64)
    }

    /// Crée un événement
    pub async fn create_event(
        &self,
        name: String,
        date_time: Option<String>,
        end_date_time: Option<String>,
        summary: Option<String>,
        comments: Option<String>,
        location: Option<String>,
        cotation: Option<OtanCotation>,
    ) -> Result<i64, String> {
        info!("Création de l'événement: {}", name);

        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        let entity_id = db.conn.execute(
            "INSERT INTO events (entity_type, name, date_time, end_date_time, summary, comments, location, created_at, updated_at, cotation, is_active) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                EntityType::Event.as_str(),
                name,
                date_time,
                end_date_time,
                summary,
                comments,
                location,
                now.clone(),
                now.clone(),
                cotation.map(|c| c.to_string()).unwrap_or_default(),
                true,
            ),
        ).map_err(|e| e.to_string())?;

        // Sauvegarder la cotation
        if let Some(cotation) = cotation {
            db.conn.execute(
                "INSERT INTO cotations (entity_type, entity_id, reliability_code, credibility_code, rated_at) 
                 VALUES (?, ?, ?, ?, ?)",
                (
                    EntityType::Event.as_str(),
                    entity_id as i64,
                    cotation.reliability.to_string(),
                    cotation.credibility.to_string(),
                    now,
                ),
            ).map_err(|e| e.to_string())?;
        }

        Ok(entity_id as i64)
    }

    /// Crée un lieu
    pub async fn create_location(
        &self,
        location_type: LocationType,
        name: String,
        address: Option<String>,
        coordinates: Option<String>,
        summary: Option<String>,
        comments: Option<String>,
        cotation: Option<OtanCotation>,
    ) -> Result<i64, String> {
        info!("Création du lieu: {}", name);

        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        let entity_id = db.conn.execute(
            "INSERT INTO locations (entity_type, location_type, name, address, coordinates, summary, comments, created_at, updated_at, cotation, is_active) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                EntityType::Location.as_str(),
                location_type.as_str(),
                name,
                address,
                coordinates,
                summary,
                comments,
                now.clone(),
                now.clone(),
                cotation.map(|c| c.to_string()).unwrap_or_default(),
                true,
            ),
        ).map_err(|e| e.to_string())?;

        // Sauvegarder la cotation
        if let Some(cotation) = cotation {
            db.conn.execute(
                "INSERT INTO cotations (entity_type, entity_id, reliability_code, credibility_code, rated_at) 
                 VALUES (?, ?, ?, ?, ?)",
                (
                    EntityType::Location.as_str(),
                    entity_id as i64,
                    cotation.reliability.to_string(),
                    cotation.credibility.to_string(),
                    now,
                ),
            ).map_err(|e| e.to_string())?;
        }

        Ok(entity_id as i64)
    }

    /// Crée un objet
    pub async fn create_object(
        &self,
        object_type: ObjectType,
        name: String,
        details: Option<String>,
        comments: Option<String>,
        cotation: Option<OtanCotation>,
    ) -> Result<i64, String> {
        info!("Création de l'objet: {}", name);

        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        let entity_id = db.conn.execute(
            "INSERT INTO objects (entity_type, object_type, name, details, comments, created_at, updated_at, cotation, is_active) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                EntityType::Object.as_str(),
                object_type.as_str(),
                name,
                details,
                comments,
                now.clone(),
                now.clone(),
                cotation.map(|c| c.to_string()).unwrap_or_default(),
                true,
            ),
        ).map_err(|e| e.to_string())?;

        // Sauvegarder la cotation
        if let Some(cotation) = cotation {
            db.conn.execute(
                "INSERT INTO cotations (entity_type, entity_id, reliability_code, credibility_code, rated_at) 
                 VALUES (?, ?, ?, ?, ?)",
                (
                    EntityType::Object.as_str(),
                    entity_id as i64,
                    cotation.reliability.to_string(),
                    cotation.credibility.to_string(),
                    now,
                ),
            ).map_err(|e| e.to_string())?;
        }

        Ok(entity_id as i64)
    }

    /// Crée une source
    pub async fn create_source(
        &self,
        name: String,
        description: Option<String>,
        url: Option<String>,
        reliability: Option<char>,
        comments: Option<String>,
    ) -> Result<i64, String> {
        info!("Création de la source: {}", name);

        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        let source_id = db.conn.execute(
            "INSERT INTO sources (entity_type, name, description, url, reliability, comments, created_at, updated_at, is_active) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                EntityType::Source.as_str(),
                name,
                description,
                url,
                reliability,
                comments,
                now.clone(),
                now.clone(),
                true,
            ),
        ).map_err(|e| e.to_string())?;

        Ok(source_id as i64)
    }

    /// Récupère une entité par son ID
    pub async fn get_entity(
        &self,
        entity_type: EntityType,
        entity_id: i64,
    ) -> Result<Option<serde_json::Value>, String> {
        let db = self.db.lock().await;

        let table_name = match entity_type {
            EntityType::Individual => "individuals",
            EntityType::Organization => "organizations",
            EntityType::Event => "events",
            EntityType::Location => "locations",
            EntityType::Object => "objects",
            EntityType::Source => "sources",
        };

        let mut stmt = db.conn.prepare(&format!(
            "SELECT * FROM {} WHERE id = ?",
            table_name
        )).map_err(|e| e.to_string())?;

        let mut rows = stmt.query([entity_id]).map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let mut entity = serde_json::Map::new();
            
            for idx in 0..100 {
                let value: serde_json::Value = row.get_ref_unwrap(idx).unwrap().into();
                entity.insert(format!("column_{}", idx), value);
            }

            Ok(Some(serde_json::Value::Object(entity)))
        } else {
            Ok(None)
        }
    }

    /// Met à jour une entité
    pub async fn update_entity(
        &self,
        entity_type: EntityType,
        entity_id: i64,
        updates: serde_json::Value,
    ) -> Result<(), String> {
        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        let table_name = match entity_type {
            EntityType::Individual => "individuals",
            EntityType::Organization => "organizations",
            EntityType::Event => "events",
            EntityType::Location => "locations",
            EntityType::Object => "objects",
            EntityType::Source => "sources",
        };

        // Construire la requête de mise à jour dynamiquement
        let mut set_clauses = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(obj) = updates.as_object() {
            for (key, value) in obj {
                if key != "id" && key != "created_at" && key != "entity_type" {
                    set_clauses.push(format!("{} = ?", key));
                    values.push(Box::new(value.as_str().unwrap_or("")) as Box<dyn rusqlite::ToSql>);
                }
            }
        }

        set_clauses.push("updated_at = ?".to_string());
        values.push(Box::new(now.clone()) as Box<dyn rusqlite::ToSql>);
        values.push(Box::new(entity_id) as Box<dyn rusqlite::ToSql>);

        let query = format!(
            "UPDATE {} SET {} WHERE id = ?",
            table_name,
            set_clauses.join(", ")
        );

        db.conn.execute(&query, values.as_slice())
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Supprime une entité
    pub async fn delete_entity(
        &self,
        entity_type: EntityType,
        entity_id: i64,
    ) -> Result<(), String> {
        let db = self.db.lock().await;

        let table_name = match entity_type {
            EntityType::Individual => "individuals",
            EntityType::Organization => "organizations",
            EntityType::Event => "events",
            EntityType::Location => "locations",
            EntityType::Object => "objects",
            EntityType::Source => "sources",
        };

        db.conn.execute(
            &format!("DELETE FROM {} WHERE id = ?", table_name),
            [entity_id],
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Recherche des entités
    pub async fn search_entities(
        &self,
        query: &str,
        entity_type: Option<EntityType>,
        limit: Option<usize>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let db = self.db.lock().await;

        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();

        if let Some(entity_type) = entity_type {
            let table_name = match entity_type {
                EntityType::Individual => "individuals",
                EntityType::Organization => "organizations",
                EntityType::Event => "events",
                EntityType::Location => "locations",
                EntityType::Object => "objects",
                EntityType::Source => "sources",
            };

            let query_str = format!(
                "SELECT * FROM {} WHERE name LIKE ? OR description LIKE ?{}",
                table_name, limit_clause
            );

            let search_pattern = format!("%{}%", query);

            let mut stmt = db.conn.prepare(&query_str).map_err(|e| e.to_string())?;
            let mut rows = stmt.query([&search_pattern, &search_pattern])
                .map_err(|e| e.to_string())?;

            let mut results = Vec::new();

            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                let mut entity = serde_json::Map::new();
                
                for idx in 0..100 {
                    let value: serde_json::Value = row.get_ref_unwrap(idx).unwrap().into();
                    entity.insert(format!("column_{}", idx), value);
                }

                results.push(serde_json::Value::Object(entity));
            }

            Ok(results)
        } else {
            // Rechercher dans toutes les tables
            let tables = ["individuals", "organizations", "events", "locations", "objects", "sources"];
            let mut all_results = Vec::new();

            for table in tables {
                let query_str = format!(
                    "SELECT * FROM {} WHERE name LIKE ? OR description LIKE ?{}",
                    table, limit_clause
                );

                let search_pattern = format!("%{}%", query);

                let mut stmt = db.conn.prepare(&query_str).map_err(|e| e.to_string())?;
                let mut rows = stmt.query([&search_pattern, &search_pattern])
                    .map_err(|e| e.to_string())?;

                while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                    let mut entity = serde_json::Map::new();
                    
                    for idx in 0..100 {
                        let value: serde_json::Value = row.get_ref_unwrap(idx).unwrap().into();
                        entity.insert(format!("column_{}", idx), value);
                    }

                    all_results.push(serde_json::Value::Object(entity));
                }
            }

            Ok(all_results)
        }
    }

    /// Lie une entité à une autre
    pub async fn link_entities(
        &self,
        source_type: EntityType,
        source_id: i64,
        target_type: EntityType,
        target_id: i64,
        relation: &str,
    ) -> Result<i64, String> {
        let db = self.db.lock().await;
        let now = chrono::Utc::now().to_rfc3339();

        let link_id = db.conn.execute(
            "INSERT INTO entity_links (source_type, source_id, target_type, target_id, relation, created_at) 
             VALUES (?, ?, ?, ?, ?, ?)",
            (
                source_type.as_str(),
                source_id,
                target_type.as_str(),
                target_id,
                relation,
                now,
            ),
        ).map_err(|e| e.to_string())?;

        Ok(link_id as i64)
    }

    /// Récupère les liens d'une entité
    pub async fn get_entity_links(
        &self,
        entity_type: EntityType,
        entity_id: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let db = self.db.lock().await;

        let query = format!(
            "SELECT * FROM entity_links 
             WHERE (source_type = ? AND source_id = ?) 
                OR (target_type = ? AND target_id = ?)"
        );

        let mut stmt = db.conn.prepare(&query).map_err(|e| e.to_string())?;
        let mut rows = stmt.query([
            entity_type.as_str(),
            entity_id,
            entity_type.as_str(),
            entity_id,
        ]).map_err(|e| e.to_string())?;

        let mut results = Vec::new();

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let mut link = serde_json::Map::new();
            
            for idx in 0..100 {
                let value: serde_json::Value = row.get_ref_unwrap(idx).unwrap().into();
                link.insert(format!("column_{}", idx), value);
            }

            results.push(serde_json::Value::Object(link));
        }

        Ok(results)
    }

    /// Enrichit une entité avec des informations depuis le web
    pub async fn enrich_entity_from_web(
        &self,
        entity_type: EntityType,
        entity_id: i64,
    ) -> Result<serde_json::Value, String> {
        // Récupérer l'entité
        let entity = self.get_entity(entity_type.clone(), entity_id).await.map_err(|e| e.to_string())?
            .ok_or("Entité non trouvée")?;

        let name = entity.get("name").and_then(|v| v.as_str()).unwrap_or_default();

        // Rechercher sur Wikipedia
        let wikipedia_info = self.search_wikipedia(name).await?;

        // Rechercher sur les réseaux sociaux
        let social_info = self.search_social_media(name).await?;

        // Combiner les informations
        let mut enriched_data = serde_json::Map::new();
        enriched_data.insert("wikipedia".to_string(), wikipedia_info);
        enriched_data.insert("social_media".to_string(), social_info);

        Ok(serde_json::Value::Object(enriched_data))
    }

    /// Recherche sur Wikipedia
    async fn search_wikipedia(&self, query: &str) -> Result<serde_json::Value, String> {
        // Pour l'instant, retourner un exemple
        // En production, utiliser l'API Wikipedia ou le scraping
        Ok(json!({
            "found": false,
            "url": format!("https://fr.wikipedia.org/wiki/{}", query),
            "summary": "",
            "extract": ""
        }))
    }

    /// Recherche sur les réseaux sociaux
    async fn search_social_media(&self, query: &str) -> Result<serde_json::Value, String> {
        // Pour l'instant, retourner un exemple
        Ok(json!({
            "twitter": null,
            "linkedin": null,
            "facebook": null,
            "instagram": null
        }))
    }
}

/// Commandes Tauri pour les entités


#[allow(dead_code)]
pub async fn create_entity(
    
    
        
        
            
        state: &crate::AppState,
    entity_type: String,
    name: String,
    description: Option<String>,
    cotation: Option<String>,
) -> Result<i64, String> {
    let entity_manager = state.entity_manager.lock().await;
    let entity_type_enum = EntityType::from_str(&entity_type)
        .ok_or_else(|| format!("Type d'entité inconnu: {}", entity_type))?;
    
    let cotation_enum = cotation.and_then(|c| OtanCotation::from_str(&c).ok());

    entity_manager.create_entity(entity_type_enum, name, description, cotation_enum).await
}


#[allow(dead_code)]
pub async fn create_individual(
    
    
        
        
            
        state: &crate::AppState,
    individual_type: String,
    first_name: Option<String>,
    last_name: Option<String>,
    full_name: String,
    biography: Option<String>,
    comments: Option<String>,
    date_of_birth: Option<String>,
    place_of_birth: Option<String>,
    nationality: Option<String>,
    linked_real_individual_id: Option<i64>,
    cotation: Option<String>,
) -> Result<i64, String> {
    let entity_manager = state.entity_manager.lock().await;
    let individual_type_enum = IndividualType::from_str(&individual_type)
        .ok_or_else(|| format!("Type d'individu inconnu: {}", individual_type))?;
    
    let cotation_enum = cotation.and_then(|c| OtanCotation::from_str(&c).ok());

    entity_manager.create_individual(
        individual_type_enum,
        first_name,
        last_name,
        full_name,
        biography,
        comments,
        date_of_birth,
        place_of_birth,
        nationality,
        linked_real_individual_id,
        cotation_enum,
    ).await
}


#[allow(dead_code)]
pub async fn create_organization(
    
    
        
        
            
        state: &crate::AppState,
    name: String,
    organization_type: String,
    summary: Option<String>,
    comments: Option<String>,
    founded_date: Option<String>,
    headquarters: Option<String>,
    website: Option<String>,
    cotation: Option<String>,
) -> Result<i64, String> {
    let entity_manager = state.entity_manager.lock().await;
    let organization_type_enum = OrganizationType::from_str(&organization_type)
        .ok_or_else(|| format!("Type d'organisation inconnu: {}", organization_type))?;
    
    let cotation_enum = cotation.and_then(|c| OtanCotation::from_str(&c).ok());

    entity_manager.create_organization(
        name,
        organization_type_enum,
        summary,
        comments,
        founded_date,
        headquarters,
        website,
        cotation_enum,
    ).await
}


#[allow(dead_code)]
pub async fn create_event(
    
    
        
        
            
        state: &crate::AppState,
    name: String,
    date_time: Option<String>,
    end_date_time: Option<String>,
    summary: Option<String>,
    comments: Option<String>,
    location: Option<String>,
    cotation: Option<String>,
) -> Result<i64, String> {
    let entity_manager = state.entity_manager.lock().await;
    let cotation_enum = cotation.and_then(|c| OtanCotation::from_str(&c).ok());

    entity_manager.create_event(
        name,
        date_time,
        end_date_time,
        summary,
        comments,
        location,
        cotation_enum,
    ).await
}


#[allow(dead_code)]
pub async fn create_location(
    
    
        
        
            
        state: &crate::AppState,
    location_type: String,
    name: String,
    address: Option<String>,
    coordinates: Option<String>,
    summary: Option<String>,
    comments: Option<String>,
    cotation: Option<String>,
) -> Result<i64, String> {
    let entity_manager = state.entity_manager.lock().await;
    let location_type_enum = LocationType::from_str(&location_type)
        .ok_or_else(|| format!("Type de lieu inconnu: {}", location_type))?;
    
    let cotation_enum = cotation.and_then(|c| OtanCotation::from_str(&c).ok());

    entity_manager.create_location(
        location_type_enum,
        name,
        address,
        coordinates,
        summary,
        comments,
        cotation_enum,
    ).await
}


#[allow(dead_code)]
pub async fn create_object(
    
    
        
        
            
        state: &crate::AppState,
    object_type: String,
    name: String,
    details: Option<String>,
    comments: Option<String>,
    cotation: Option<String>,
) -> Result<i64, String> {
    let entity_manager = state.entity_manager.lock().await;
    let object_type_enum = ObjectType::from_str(&object_type)
        .ok_or_else(|| format!("Type d'objet inconnu: {}", object_type))?;
    
    let cotation_enum = cotation.and_then(|c| OtanCotation::from_str(&c).ok());

    entity_manager.create_object(
        object_type_enum,
        name,
        details,
        comments,
        cotation_enum,
    ).await
}


#[allow(dead_code)]
pub async fn create_source(
    
    
        
        
            
        state: &crate::AppState,
    name: String,
    description: Option<String>,
    url: Option<String>,
    reliability: Option<char>,
    comments: Option<String>,
) -> Result<i64, String> {
    let entity_manager = state.entity_manager.lock().await;
    entity_manager.create_source(name, description, url, reliability, comments).await
}


#[allow(dead_code)]
pub async fn get_entity(
    
    
        
        
            
        state: &crate::AppState,
    entity_type: String,
    entity_id: i64,
) -> Result<Option<serde_json::Value>, String> {
    let entity_manager = state.entity_manager.lock().await;
    let entity_type_enum = EntityType::from_str(&entity_type)
        .ok_or_else(|| format!("Type d'entité inconnu: {}", entity_type))?;
    
    entity_manager.get_entity(entity_type_enum, entity_id).await
}


#[allow(dead_code)]
pub async fn search_entities(
    
    
        
        
            
        state: &crate::AppState,
    query: String,
    entity_type: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<serde_json::Value>, String> {
    let entity_manager = state.entity_manager.lock().await;
    let entity_type_enum = entity_type.and_then(|t| EntityType::from_str(&t).ok());
    
    entity_manager.search_entities(&query, entity_type_enum, limit).await
}


#[allow(dead_code)]
pub async fn link_entities(
    
    
        
        
            
        state: &crate::AppState,
    source_type: String,
    source_id: i64,
    target_type: String,
    target_id: i64,
    relation: String,
) -> Result<i64, String> {
    let entity_manager = state.entity_manager.lock().await;
    let source_type_enum = EntityType::from_str(&source_type)
        .ok_or_else(|| format!("Type d'entité source inconnu: {}", source_type))?;
    let target_type_enum = EntityType::from_str(&target_type)
        .ok_or_else(|| format!("Type d'entité cible inconnu: {}", target_type))?;
    
    entity_manager.link_entities(source_type_enum, source_id, target_type_enum, target_id, &relation).await
}


#[allow(dead_code)]
pub async fn get_entity_links(
    
    
        
        
            
        state: &crate::AppState,
    entity_type: String,
    entity_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let entity_manager = state.entity_manager.lock().await;
    let entity_type_enum = EntityType::from_str(&entity_type)
        .ok_or_else(|| format!("Type d'entité inconnu: {}", entity_type))?;
    
    entity_manager.get_entity_links(entity_type_enum, entity_id).await
}


#[allow(dead_code)]
pub async fn enrich_entity(
    
    
        
        
            
        state: &crate::AppState,
    entity_type: String,
    entity_id: i64,
) -> Result<serde_json::Value, String> {
    let entity_manager = state.entity_manager.lock().await;
    let entity_type_enum = EntityType::from_str(&entity_type)
        .ok_or_else(|| format!("Type d'entité inconnu: {}", entity_type))?;
    
    entity_manager.enrich_entity_from_web(entity_type_enum, entity_id).await
}


#[allow(dead_code)]
pub async fn get_individual_types() -> Vec<String> {
    vec![
        IndividualType::Real.as_str().to_string(),
        IndividualType::Fictive.as_str().to_string(),
        IndividualType::Numerique.as_str().to_string(),
    ]
}


#[allow(dead_code)]
pub async fn get_location_types() -> Vec<String> {
    vec![
        LocationType::Real.as_str().to_string(),
        LocationType::Fictive.as_str().to_string(),
        LocationType::Numerique.as_str().to_string(),
    ]
}


#[allow(dead_code)]
pub async fn get_organization_types() -> Vec<String> {
    vec![
        OrganizationType::Gouvernement.as_str().to_string(),
        OrganizationType::Entreprise.as_str().to_string(),
        OrganizationType::ONG.as_str().to_string(),
        OrganizationType::Media.as_str().to_string(),
        OrganizationType::Militaire.as_str().to_string(),
        OrganizationType::Criminel.as_str().to_string(),
        OrganizationType::Religieux.as_str().to_string(),
        OrganizationType::Educatif.as_str().to_string(),
        OrganizationType::Autre.as_str().to_string(),
    ]
}


#[allow(dead_code)]
pub async fn get_object_types() -> Vec<String> {
    vec![
        ObjectType::Vehicule.as_str().to_string(),
        ObjectType::Arme.as_str().to_string(),
        ObjectType::Document.as_str().to_string(),
        ObjectType::Equipement.as_str().to_string(),
        ObjectType::Bâtiment.as_str().to_string(),
        ObjectType::Autre.as_str().to_string(),
    ]
}
