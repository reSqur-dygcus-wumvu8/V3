//! Modèles de données partagés entre les applications

use serde::{Serialize, Deserialize};
use std::path::PathBuf;

/// Paramètres partagés entre les applications
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub tor: TorSettings,
    pub maps: MapSettings,
    pub api: ApiSettings,
    pub watch: WatchSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            tor: TorSettings::default(),
            maps: MapSettings::default(),
            api: ApiSettings::default(),
            watch: WatchSettings::default(),
        }
    }
}

/// Paramètres TOR
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TorSettings {
    pub enabled: bool,
    pub socks_port: u16,
    pub control_port: u16,
    pub auto_start: bool,
    pub exit_nodes: Vec<String>,
    pub data_directory: Option<PathBuf>,
}

impl Default for TorSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            socks_port: 9050,
            control_port: 9051,
            auto_start: true,
            exit_nodes: Vec::new(),
            data_directory: None,
        }
    }
}

/// Paramètres des cartes
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MapSettings {
    pub default_provider: String,
    pub cache_enabled: bool,
    pub cache_size: u32, // en Mo
    pub cache_directory: Option<PathBuf>,
}

impl Default for MapSettings {
    fn default() -> Self {
        Self {
            default_provider: "OpenStreetMap.Mapnik".to_string(),
            cache_enabled: true,
            cache_size: 500,
            cache_directory: None,
        }
    }
}

/// Paramètres des APIs externes
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiSettings {
    pub mistral_key: Option<String>,
    pub mistral_endpoint: String,
    pub acl_endpoint: String,
}

impl Default for ApiSettings {
    fn default() -> Self {
        Self {
            mistral_key: None,
            mistral_endpoint: "https://api.mistral.ai/v1".to_string(),
            acl_endpoint: "https://acleddata.com/api".to_string(),
        }
    }
}

/// Paramètres de veille
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WatchSettings {
    pub default_frequency: u32, // en minutes
    pub default_language: String,
    pub max_concurrent_watches: u32,
}

impl Default for WatchSettings {
    fn default() -> Self {
        Self {
            default_frequency: 60,
            default_language: "fr".to_string(),
            max_concurrent_watches: 5,
        }
    }
}

impl AppSettings {
    pub fn secrets_socket_path(&self) -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("osint_app");
        
        config_dir.join("secrets.sock")
    }
}

/// Types d'entités
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum EntityType {
    Individual,
    Organization,
    Event,
    Location,
    Object,
    Source,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Individual => "individual",
            EntityType::Organization => "organization",
            EntityType::Event => "event",
            EntityType::Location => "location",
            EntityType::Object => "object",
            EntityType::Source => "source",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "individual" => Some(EntityType::Individual),
            "organization" => Some(EntityType::Organization),
            "event" => Some(EntityType::Event),
            "location" => Some(EntityType::Location),
            "object" => Some(EntityType::Object),
            "source" => Some(EntityType::Source),
            _ => None,
        }
    }
}

/// Cotations OTAN
pub const RELIABILITY_CODES: [(&str, &str); 6] = [
    ("A", "Totalement fiable"),
    ("B", "Généralement fiable"),
    ("C", "Assez fiable"),
    ("D", "Peu fiable"),
    ("E", "Fiabilité non évaluable"),
    ("F", "Non évaluable"),
];

pub const CREDIBILITY_CODES: [(&str, &str); 6] = [
    ("1", "Confirmée"),
    ("2", "Probablement vraie"),
    ("3", "Possiblement vraie"),
    ("4", "Douteuse"),
    ("5", "Improbable"),
    ("6", "Non évaluable"),
];
