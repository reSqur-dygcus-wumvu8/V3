use crate::AppState;
// Module de gestion de la base de données SQLite avec chiffrement SQLCipher + MLA

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use rusqlite::{Connection, OpenFlags, Result as SqliteResult};
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::backend::settings::AppSettings;

/// Chemin par défaut de la base de données
pub const DEFAULT_DB_PATH: &str = "osint_app.db";

/// Clé SQLCipher par défaut (à remplacer par une clé générée)
pub const DEFAULT_SQLCIPHER_KEY: &str = "default-key-change-me";

/// Contexte de la base de données
pub struct Database {
    pub conn: Connection,
    pub path: PathBuf,
    pub mla_enabled: bool,
}

impl Database {
    /// Crée une nouvelle connexion à la base de données
    pub async fn new(settings: &AppSettings) -> Result<Self> {
        // Déterminer le chemin de la base
        let db_path = settings.maps.cache_directory
            .clone()
            .unwrap_or_else(|| {
                dirs::data_dir()
                    .map(|p| p.join("osint_app").join(DEFAULT_DB_PATH))
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_DB_PATH))
            });
        
        // Créer le répertoire parent si nécessaire
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Ouvrir la connexion SQLite
        let mut conn = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
        
        // Activer SQLCipher (si disponible)
        // Note: SQLCipher doit être compilé avec SQLite
        conn.execute_batch(
            "PRAGMA key = 'test-key'; \
             PRAGMA cipher_page_size = 4096; \
             PRAGMA kdf_iter = 256000; \
             PRAGMA cipher_hmac_algorithm = HMAC_SHA512; \
             PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512;"
        ).ok(); // Ignorer l'erreur si SQLCipher n'est pas disponible
        
        // Initialiser les tables
        Self::init_tables(&conn)?;
        
        Ok(Self {
            conn,
            path: db_path,
            mla_enabled: false, // À activer après l'intégration de MLA
        })
    }
    
    /// Initialise les tables de la base de données
    fn init_tables(conn: &Connection) -> Result<()> {
        // Types d'entités
        conn.execute_batch(
            "-- Types d'entités
             CREATE TABLE IF NOT EXISTS entity_types (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL UNIQUE,
                 label_fr TEXT NOT NULL
             );
             
             -- Types de lieux
             CREATE TABLE IF NOT EXISTS location_types (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL UNIQUE,
                 label_fr TEXT NOT NULL
             );
             
             -- Types d'individus
             CREATE TABLE IF NOT EXISTS individual_types (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL UNIQUE,
                 label_fr TEXT NOT NULL
             );
             
             -- Types d'organisations
             CREATE TABLE IF NOT EXISTS organization_types (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL UNIQUE,
                 label_fr TEXT NOT NULL
             );
             
             -- Types d'objets
             CREATE TABLE IF NOT EXISTS object_types (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL UNIQUE,
                 label_fr TEXT NOT NULL
             );
             
             -- Fiabilité des sources (A-F)
             CREATE TABLE IF NOT EXISTS source_reliability (
                 code TEXT PRIMARY KEY,
                 label_fr TEXT NOT NULL,
                 order_val INTEGER NOT NULL
             );
             
             -- Crédibilité de l'information (1-6)
             CREATE TABLE IF NOT EXISTS info_credibility (
                 code INTEGER PRIMARY KEY,
                 label_fr TEXT NOT NULL,
                 order_val INTEGER NOT NULL
             );
             
             -- Individus
             CREATE TABLE IF NOT EXISTS individuals (
                 id INTEGER PRIMARY KEY,
                 type_id INTEGER NOT NULL REFERENCES individual_types(id),
                 first_name TEXT,
                 last_name TEXT,
                 biography TEXT,
                 comments TEXT,
                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 validated BOOLEAN DEFAULT FALSE,
                 wikipedia_url TEXT
             );
             
             -- Liens entre individus (fictif/numérique -> réel)
             CREATE TABLE IF NOT EXISTS individual_links (
                 child_id INTEGER NOT NULL REFERENCES individuals(id),
                 parent_id INTEGER NOT NULL REFERENCES individuals(id),
                 PRIMARY KEY (child_id, parent_id)
             );
             
             -- Organisations
             CREATE TABLE IF NOT EXISTS organizations (
                 id INTEGER PRIMARY KEY,
                 type_id INTEGER NOT NULL REFERENCES organization_types(id),
                 name TEXT NOT NULL,
                 summary TEXT,
                 comments TEXT,
                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 validated BOOLEAN DEFAULT FALSE
             );
             
             -- Événements
             CREATE TABLE IF NOT EXISTS events (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL,
                 start_date TIMESTAMP,
                 end_date TIMESTAMP,
                 summary TEXT,
                 comments TEXT,
                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 validated BOOLEAN DEFAULT FALSE
             );
             
             -- Lieux
             CREATE TABLE IF NOT EXISTS locations (
                 id INTEGER PRIMARY KEY,
                 type_id INTEGER NOT NULL REFERENCES location_types(id),
                 name TEXT NOT NULL,
                 address TEXT,
                 gps_lat REAL,
                 gps_lon REAL,
                 ip_address TEXT,
                 summary TEXT,
                 comments TEXT,
                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 validated BOOLEAN DEFAULT FALSE
             );
             
             -- Objets
             CREATE TABLE IF NOT EXISTS objects (
                 id INTEGER PRIMARY KEY,
                 type_id INTEGER NOT NULL REFERENCES object_types(id),
                 name TEXT NOT NULL,
                 details TEXT,
                 comments TEXT,
                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 validated BOOLEAN DEFAULT FALSE
             );
             
             -- Sources
             CREATE TABLE IF NOT EXISTS sources (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL,
                 description TEXT,
                 url TEXT,
                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
             );
             
             -- Cotation des SOURCES
             CREATE TABLE IF NOT EXISTS source_ratings (
                 id INTEGER PRIMARY KEY,
                 source_id INTEGER NOT NULL REFERENCES sources(id),
                 reliability_code TEXT NOT NULL REFERENCES source_reliability(code),
                 rated_by INTEGER,
                 rated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 UNIQUE (source_id, rated_by)
             );
             
             -- Cotation des INFORMATIONS
             CREATE TABLE IF NOT EXISTS info_ratings (
                 id INTEGER PRIMARY KEY,
                 entity_type TEXT NOT NULL,
                 entity_id INTEGER NOT NULL,
                 info_field TEXT NOT NULL,
                 credibility_code INTEGER NOT NULL REFERENCES info_credibility(code),
                 source_id INTEGER REFERENCES sources(id),
                 rated_by INTEGER,
                 rated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 UNIQUE (entity_type, entity_id, info_field, source_id, rated_by)
             );
             
             -- Configuration des veilles
             CREATE TABLE IF NOT EXISTS watches (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL,
                 query TEXT NOT NULL,
                 language TEXT NOT NULL DEFAULT 'fr',
                 sources TEXT NOT NULL,
                 frequency_minutes INTEGER NOT NULL,
                 is_active BOOLEAN NOT NULL DEFAULT TRUE,
                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
             );
             
             -- Résultats des veilles
             CREATE TABLE IF NOT EXISTS watch_results (
                 id INTEGER PRIMARY KEY,
                 watch_id INTEGER NOT NULL REFERENCES watches(id),
                 title TEXT NOT NULL,
                 url TEXT NOT NULL,
                 content TEXT NOT NULL,
                 translated_content TEXT,
                 source_name TEXT NOT NULL,
                 published_at TIMESTAMP,
                 fetched_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 is_processed BOOLEAN DEFAULT FALSE,
                 metadata TEXT
             );
             
             -- Liens entre résultats de veille et entités
             CREATE TABLE IF NOT EXISTS watch_result_entities (
                 result_id INTEGER NOT NULL REFERENCES watch_results(id),
                 entity_type TEXT NOT NULL,
                 entity_id INTEGER NOT NULL,
                 confidence REAL NOT NULL,
                 PRIMARY KEY (result_id, entity_type, entity_id)
             );
             
             -- Informations capitalisées
             CREATE TABLE IF NOT EXISTS capitalized_infos (
                 id INTEGER PRIMARY KEY,
                 title TEXT NOT NULL,
                 content TEXT NOT NULL,
                 event_id INTEGER REFERENCES events(id),
                 source_id INTEGER NOT NULL REFERENCES sources(id),
                 watch_result_id INTEGER REFERENCES watch_results(id),
                 file_id INTEGER REFERENCES imported_files(id),
                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 validated BOOLEAN DEFAULT FALSE
             );
             
             -- Liens entre informations capitalisées et entités
             CREATE TABLE IF NOT EXISTS capitalized_info_entities (
                 info_id INTEGER NOT NULL REFERENCES capitalized_infos(id),
                 entity_type TEXT NOT NULL,
                 entity_id INTEGER NOT NULL,
                 PRIMARY KEY (info_id, entity_type, entity_id)
             );
             
             -- Import de fichiers
             CREATE TABLE IF NOT EXISTS imported_files (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL,
                 path TEXT NOT NULL,
                 import_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 source_id INTEGER REFERENCES sources(id),
                 hash TEXT UNIQUE NOT NULL
             );
             
             -- Relations entre entités
             CREATE TABLE IF NOT EXISTS entity_relations (
                 id INTEGER PRIMARY KEY,
                 from_type TEXT NOT NULL,
                 from_id INTEGER NOT NULL,
                 to_type TEXT NOT NULL,
                 to_id INTEGER NOT NULL,
                 relation_type TEXT NOT NULL,
                 description TEXT,
                 source_id INTEGER REFERENCES sources(id),
                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 UNIQUE (from_type, from_id, to_type, to_id, relation_type)
             );
             
             -- Historique / Audit
             CREATE TABLE IF NOT EXISTS audit_log (
                 id INTEGER PRIMARY KEY,
                 action TEXT NOT NULL,
                 entity_type TEXT,
                 entity_id INTEGER,
                 field TEXT,
                 old_value TEXT,
                 new_value TEXT,
                 user_id INTEGER,
                 timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 ip_address TEXT,
                 tor_exit_node TEXT
             );
             
             -- Comptes réseaux sociaux
             CREATE TABLE IF NOT EXISTS social_accounts (
                 id INTEGER PRIMARY KEY,
                 individual_id INTEGER NOT NULL REFERENCES individuals(id),
                 platform TEXT NOT NULL,
                 username TEXT NOT NULL,
                 url TEXT,
                 verified BOOLEAN DEFAULT FALSE,
                 UNIQUE (individual_id, platform, username)
             );
             
             -- Groupes de doublons
             CREATE TABLE IF NOT EXISTS duplicate_groups (
                 id INTEGER PRIMARY KEY,
                 similarity_score REAL NOT NULL,
                 status TEXT NOT NULL DEFAULT 'pending',
                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 resolved_at TIMESTAMP,
                 resolution_type TEXT
             );
             
             -- Entités dans les groupes de doublons
             CREATE TABLE IF NOT EXISTS duplicate_group_entities (
                 group_id INTEGER NOT NULL REFERENCES duplicate_groups(id),
                 entity_type TEXT NOT NULL,
                 entity_id INTEGER NOT NULL,
                 PRIMARY KEY (group_id, entity_type, entity_id)
             );
             
             -- Cache des données ACLED
             CREATE TABLE IF NOT EXISTS acl_cache (
                 id INTEGER PRIMARY KEY,
                 event_id TEXT NOT NULL,
                 event_date TEXT NOT NULL,
                 event_type TEXT NOT NULL,
                 latitude REAL,
                 longitude REAL,
                 location TEXT,
                 actor1 TEXT,
                 actor2 TEXT,
                 fatalities INTEGER,
                 reliability TEXT,
                 credibility TEXT,
                 fetched_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                 UNIQUE (event_id)
             );
             
             -- Paramètres de l'application
             CREATE TABLE IF NOT EXISTS app_settings (
                 id INTEGER PRIMARY KEY,
                 key TEXT NOT NULL UNIQUE,
                 value TEXT
             );"
        )?;
        
        // Insérer les types d'entités par défaut
        conn.execute_batch(
            "-- Types d'entités
             INSERT OR IGNORE INTO entity_types (id, name, label_fr) VALUES
                 (1, 'individual', 'Individu'),
                 (2, 'organization', 'Organisation'),
                 (3, 'event', 'Événement'),
                 (4, 'location', 'Lieu'),
                 (5, 'object', 'Objet'),
                 (6, 'source', 'Source');
             
             -- Types de lieux
             INSERT OR IGNORE INTO location_types (id, name, label_fr) VALUES
                 (1, 'real', 'Réel'),
                 (2, 'fictive', 'Fictif'),
                 (3, 'digital', 'Numérique');
             
             -- Types d'individus
             INSERT OR IGNORE INTO individual_types (id, name, label_fr) VALUES
                 (1, 'real', 'Identité réelle'),
                 (2, 'fictive', 'Identité fictive'),
                 (3, 'digital', 'Identité numérique');
             
             -- Types d'organisations (exemples)
             INSERT OR IGNORE INTO organization_types (id, name, label_fr) VALUES
                 (1, 'government', 'Gouvernement'),
                 (2, 'company', 'Entreprise'),
                 (3, 'ngo', 'ONG'),
                 (4, 'military', 'Militaire'),
                 (5, 'criminal', 'Criminelle'),
                 (6, 'media', 'Média'),
                 (7, 'religious', 'Religieuse'),
                 (8, 'educational', 'Éducative');
             
             -- Types d'objets (exemples)
             INSERT OR IGNORE INTO object_types (id, name, label_fr) VALUES
                 (1, 'vehicle', 'Véhicule'),
                 (2, 'weapon', 'Arme'),
                 (3, 'document', 'Document'),
                 (4, 'device', 'Appareil'),
                 (5, 'building', 'Bâtiment');
             
             -- Fiabilité des sources (A-F)
             INSERT OR IGNORE INTO source_reliability (code, label_fr, order_val) VALUES
                 ('A', 'Totalement fiable', 1),
                 ('B', 'Généralement fiable', 2),
                 ('C', 'Assez fiable', 3),
                 ('D', 'Peu fiable', 4),
                 ('E', 'Fiabilité non évaluable', 5),
                 ('F', 'Non évaluable', 6);
             
             -- Crédibilité de l'information (1-6)
             INSERT OR IGNORE INTO info_credibility (code, label_fr, order_val) VALUES
                 (1, 'Confirmée', 1),
                 (2, 'Probablement vraie', 2),
                 (3, 'Possiblement vraie', 3),
                 (4, 'Douteuse', 4),
                 (5, 'Improbable', 5),
                 (6, 'Non évaluable', 6);"
        )?;
        
        Ok(())
    }
    
    /// Exécute une requête et retourne les résultats
    pub async fn query<T, F>(&self, sql: &str, params: T, mut callback: F) -> SqliteResult<()>
    where
        T: rusqlite::Params,
        F: FnMut(&rusqlite::Row) -> rusqlite::Result<()>
    {
        self.conn.prepare(sql)?.query_map(params, |row| callback(row))?;
        Ok(())
    }
    
    /// Récupère une entité par son ID et son type
    pub async fn get_entity(&self, entity_type: &str, id: i64) -> Result<Option<serde_json::Value>> {
        let table = match entity_type {
            "individual" => "individuals",
            "organization" => "organizations",
            "event" => "events",
            "location" => "locations",
            "object" => "objects",
            "source" => "sources",
            _ => return Err(anyhow::anyhow!("Type d'entité inconnu: {}", entity_type)),
        };
        
        let mut stmt = self.conn.prepare(&format!("SELECT * FROM {} WHERE id = ?", table))?;
        let column_count = stmt.column_count();
        let mut rows = stmt.query([id])?;
        
        if let Some(row) = rows.next()? {
            let mut entity = serde_json::Map::new();
            // Ajouter les colonnes dynamiquement
            for i in 0..column_count {
                let col_name = stmt.column_name(i).unwrap();
                
                // Try to get value as JSON Value
                if let Ok(value) = row.get::<_, serde_json::Value>(i) {
                    entity.insert(col_name.to_string(), value);
                } else if let Ok(value) = row.get::<_, i64>(i) {
                    entity.insert(col_name.to_string(), serde_json::Value::from(value));
                } else if let Ok(value) = row.get::<_, String>(i) {
                    entity.insert(col_name.to_string(), serde_json::Value::from(value));
                } else if let Ok(value) = row.get::<_, f64>(i) {
                    entity.insert(col_name.to_string(), serde_json::Value::from(value));
                }
            }
            entity.insert("type".to_string(), serde_json::Value::from(entity_type));
            entity.insert("id".to_string(), serde_json::Value::from(id));
            Ok(Some(serde_json::Value::Object(entity)))
        } else {
            Ok(None)
        }
    }
    
    /// Ajoute une entité
    pub async fn add_entity(&self, entity_type: &str, data: &serde_json::Map<String, serde_json::Value>) -> Result<i64> {
        let table = match entity_type {
            "individual" => "individuals",
            "organization" => "organizations",
            "event" => "events",
            "location" => "locations",
            "object" => "objects",
            "source" => "sources",
            _ => return Err(anyhow::anyhow!("Type d'entité inconnu: {}", entity_type)),
        };
        
        // Construire la requête INSERT
        let mut columns = Vec::new();
        let mut values = Vec::new();
        let mut placeholders = Vec::new();
        
        for (key, value) in data.iter() {
            if key == "id" || key == "type" {
                continue;
            }
            columns.push(key.clone());
            placeholders.push("?".to_string());
            match value {
                serde_json::Value::Number(n) => {
                    if n.is_i64() {
                        values.push(rusqlite::types::Value::Integer(n.as_i64().unwrap()));
                    } else if n.is_u64() {
                        values.push(rusqlite::types::Value::Integer(n.as_i64().unwrap()));
                    } else if n.is_f64() {
                        values.push(rusqlite::types::Value::Real(n.as_f64().unwrap()));
                    }
                }
                serde_json::Value::String(s) => {
                    values.push(rusqlite::types::Value::Text(s.clone()));
                }
                serde_json::Value::Bool(b) => {
                    values.push(rusqlite::types::Value::Integer(if *b { 1 } else { 0 }));
                }
                serde_json::Value::Null => {
                    values.push(rusqlite::types::Value::Null);
                }
                _ => {
                    values.push(rusqlite::types::Value::Text(value.to_string()));
                }
            }
        }
        
        // Ajouter created_at et updated_at
        let now = Utc::now().to_rfc3339();
        columns.push("created_at".to_string());
        placeholders.push("?".to_string());
        values.push(rusqlite::types::Value::Text(now.clone()));
        
        columns.push("updated_at".to_string());
        placeholders.push("?".to_string());
        values.push(rusqlite::types::Value::Text(now));
        
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            columns.join(", "),
            placeholders.join(", ")
        );
        
        let row_id = self.conn.execute(&sql, rusqlite::params_from_iter(values))?;
        
        Ok(row_id as i64)
    }
    
    /// Met à jour une entité
    pub async fn update_entity(&self, entity_type: &str, id: i64, data: &serde_json::Map<String, serde_json::Value>) -> Result<()> {
        let table = match entity_type {
            "individual" => "individuals",
            "organization" => "organizations",
            "event" => "events",
            "location" => "locations",
            "object" => "objects",
            "source" => "sources",
            _ => return Err(anyhow::anyhow!("Type d'entité inconnu: {}", entity_type)),
        };
        
        let mut updates = Vec::new();
        let mut values = Vec::new();
        
        for (key, value) in data.iter() {
            if key == "id" || key == "type" || key == "created_at" {
                continue;
            }
            match value {
                serde_json::Value::Number(n) => {
                    if n.is_i64() {
                        updates.push(format!("{} = ?", key));
                        values.push(rusqlite::types::Value::Integer(n.as_i64().unwrap()));
                    } else if n.is_u64() {
                        updates.push(format!("{} = ?", key));
                        values.push(rusqlite::types::Value::Integer(n.as_i64().unwrap()));
                    } else if n.is_f64() {
                        updates.push(format!("{} = ?", key));
                        values.push(rusqlite::types::Value::Real(n.as_f64().unwrap()));
                    }
                }
                serde_json::Value::String(s) => {
                    updates.push(format!("{} = ?", key));
                    values.push(rusqlite::types::Value::Text(s.clone()));
                }
                serde_json::Value::Bool(b) => {
                    updates.push(format!("{} = ?", key));
                    values.push(rusqlite::types::Value::Integer(if *b { 1 } else { 0 }));
                }
                serde_json::Value::Null => {
                    updates.push(format!("{} = NULL", key));
                }
                _ => {
                    updates.push(format!("{} = ?", key));
                    values.push(rusqlite::types::Value::Text(value.to_string()));
                }
            }
        }
        
        // Ajouter updated_at
        updates.push("updated_at = ?".to_string());
        let now = Utc::now().to_rfc3339();
        values.push(rusqlite::types::Value::Text(now));
        
        let sql = format!(
            "UPDATE {} SET {} WHERE id = ?",
            table,
            updates.join(", ")
        );
        
        values.push(rusqlite::types::Value::Integer(id));
        
        self.conn.execute(&sql, rusqlite::params_from_iter(values))?;
        
        Ok(())
    }
    
    /// Supprime une entité
    pub async fn delete_entity(&self, entity_type: &str, id: i64) -> Result<()> {
        let table = match entity_type {
            "individual" => "individuals",
            "organization" => "organizations",
            "event" => "events",
            "location" => "locations",
            "object" => "objects",
            "source" => "sources",
            _ => return Err(anyhow::anyhow!("Type d'entité inconnu: {}", entity_type)),
        };
        
        let sql = format!("DELETE FROM {} WHERE id = ?", table);
        self.conn.execute(&sql, [id])?;
        
        Ok(())
    }
    
    /// Récupère toutes les veilles actives
    pub async fn get_active_watches(&self) -> Result<Vec<serde_json::Value>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM watches WHERE is_active = 1"
        )?;
        
        let mut watches = Vec::new();
        let mut rows = stmt.query([])?;
        
        while let Some(row) = rows.next()? {
            let mut watch = serde_json::Map::new();for i in 0..stmt.column_count() {
                let col_name = stmt.column_name(i).unwrap();
                let value: serde_json::Value = match "TEXT" {
                    t if t.contains("INTEGER") => {
                        row.get::<_, i64>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    t if t.contains("TEXT") => {
                        row.get::<_, String>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    t if t.contains("REAL") => {
                        row.get::<_, f64>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    t if t.contains("BOOLEAN") => {
                        row.get::<_, bool>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    _ => {
                        row.get::<_, String>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                };
                
                watch.insert(col_name.to_string(), value);
            }
            watches.push(serde_json::Value::Object(watch));
        }
        
        Ok(watches)
    }
    
    /// Sauvegarde un résultat de veille
    pub async fn save_watch_result(&self, watch_id: i64, result: &serde_json::Map<String, serde_json::Value>) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO watch_results (watch_id, title, url, content, translated_content, source_name, published_at, fetched_at, is_processed, metadata) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )?;
        
        let row_id = stmt.execute(rusqlite::params![
            watch_id,
            result.get("title").and_then(|v| v.as_str()).unwrap_or(""),
            result.get("url").and_then(|v| v.as_str()).unwrap_or(""),
            result.get("content").and_then(|v| v.as_str()).unwrap_or(""),
            result.get("translated_content").and_then(|v| v.as_str()).unwrap_or(""),
            result.get("source_name").and_then(|v| v.as_str()).unwrap_or(""),
            result.get("published_at").and_then(|v| v.as_str()).unwrap_or(""),
            Utc::now().to_rfc3339(),
            false,
            result.get("metadata").and_then(|v| v.as_str()).unwrap_or(""),
        ])?;
        
        Ok(row_id as i64)
    }
    
    /// Récupère les résultats de veille non traités
    pub async fn get_unprocessed_watch_results(&self) -> Result<Vec<serde_json::Value>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM watch_results WHERE is_processed = 0"
        )?;
        
        let mut results = Vec::new();
        let mut rows = stmt.query([])?;
        
        while let Some(row) = rows.next()? {
            let mut result = serde_json::Map::new();for i in 0..stmt.column_count() {
                let col_name = stmt.column_name(i).unwrap();
                let value: serde_json::Value = match "TEXT" {
                    t if t.contains("INTEGER") => {
                        row.get::<_, i64>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    t if t.contains("TEXT") => {
                        row.get::<_, String>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    t if t.contains("REAL") => {
                        row.get::<_, f64>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    t if t.contains("BOOLEAN") => {
                        row.get::<_, bool>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    _ => {
                        row.get::<_, String>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                };
                
                result.insert(col_name.to_string(), value);
            }
            results.push(serde_json::Value::Object(result));
        }
        
        Ok(results)
    }
    
    /// Marque un résultat de veille comme traité
    pub async fn mark_watch_result_processed(&self, result_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE watch_results SET is_processed = 1 WHERE id = ?",
            [result_id]
        )?;
        Ok(())
    }
    
    /// Sauvegarde les données ACLED en cache
    pub async fn cache_acled_data(&self, events: &[serde_json::Value]) -> Result<()> {
        let tx = self.conn.transaction()?;
        
        for event in events {
            let event_obj = event.as_object().ok_or_else(|| anyhow::anyhow!("Événement ACLED invalide"))?;
            let event_id = event_obj.get("event_id").and_then(|v| v.as_str()).unwrap_or("");
            let event_date = event_obj.get("event_date").and_then(|v| v.as_str()).unwrap_or("");
            let event_type = event_obj.get("event_type").and_then(|v| v.as_str()).unwrap_or("");
            let latitude = event_obj.get("latitude").and_then(|v| v.as_f64());
            let longitude = event_obj.get("longitude").and_then(|v| v.as_f64());
            let location = event_obj.get("location").and_then(|v| v.as_str()).unwrap_or("");
            let actor1 = event_obj.get("actor1").and_then(|v| v.as_str()).unwrap_or("");
            let actor2 = event_obj.get("actor2").and_then(|v| v.as_str()).unwrap_or("");
            let fatalities = event_obj.get("fatalities").and_then(|v| v.as_i64());
            let reliability = event_obj.get("reliability").and_then(|v| v.as_str()).unwrap_or("");
            let credibility = event_obj.get("credibility").and_then(|v| v.as_str()).unwrap_or("");
            tx.execute(
                "INSERT OR REPLACE INTO acl_cache (event_id, event_date, event_type, latitude, longitude, location, actor1, actor2, fatalities, reliability, credibility, fetched_at) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
                rusqlite::params![
                    event_id,
                    event_date,
                    event_type,
                    latitude,
                    longitude,
                    location,
                    actor1,
                    actor2,
                    fatalities,
                    reliability,
                    credibility,
                ]
            )?;
        }
        
        tx.commit()?;
        Ok(())
    }
    
    /// Récupère les données ACLED en cache
    pub async fn get_cached_acled_data(&self) -> Result<Vec<serde_json::Value>> {
        let mut stmt = self.conn.prepare("SELECT * FROM acl_cache")?;
        
        let mut events = Vec::new();
        let mut rows = stmt.query([])?;
        
        while let Some(row) = rows.next()? {
            let mut event = serde_json::Map::new();for i in 0..stmt.column_count() {
                let col_name = stmt.column_name(i).unwrap();
                let value: serde_json::Value = match "TEXT" {
                    t if t.contains("INTEGER") => {
                        row.get::<_, i64>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    t if t.contains("TEXT") => {
                        row.get::<_, String>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    t if t.contains("REAL") => {
                        row.get::<_, f64>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    t if t.contains("BOOLEAN") => {
                        row.get::<_, bool>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                    _ => {
                        row.get::<_, String>(i).map(|v| serde_json::Value::from(v)).unwrap_or(serde_json::Value::Null)
                    }
                };
                
                event.insert(col_name.to_string(), value);
            }
            events.push(serde_json::Value::Object(event));
        }
        
        Ok(events)
    }
    
    /// Récupère la dernière date de mise à jour du cache ACLED
    pub async fn get_last_acled_update(&self) -> Result<DateTime<Utc>> {
        let mut stmt = self.conn.prepare("SELECT MAX(fetched_at) FROM acl_cache")?;
        
        let mut rows = stmt.query([])?;
        
        if let Some(row) = rows.next()? {
            if let Ok(fetched_at) = row.get::<_, String>(0) {
                if !fetched_at.is_empty() {
                    return Ok(DateTime::parse_from_rfc3339(&fetched_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()));
                }
            }
        }
        
        Ok(Utc::now())
    }
}

/// Commandes Tauri pour la base de données


#[allow(dead_code)]
pub async fn test_db_connection(
    
        
        
            
        state: &crate::AppState) -> Result<bool, String> {
    let db = state.db.lock().await;
    
    // Tester la connexion en exécutant une requête simple
    let result = db.conn.execute_batch("SELECT 1");
    
    Ok(result.is_ok())
}
