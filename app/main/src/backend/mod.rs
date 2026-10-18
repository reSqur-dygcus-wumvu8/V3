//! Module backend de l'application principale OSINT
//! Contient toute la logique métier

pub mod db;
pub mod tor;
pub mod mla;
pub mod entities;
pub mod watch;
pub mod social;
pub mod ner;
pub mod dedup;
pub mod cotations;
pub mod api;
pub mod importer;
pub mod discovery;
pub mod capitalize;
pub mod exploit;
pub mod gerer;
pub mod settings;
