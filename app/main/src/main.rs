//! Application principale OSINT
//! Gère la collecte, capitalisation et exploitation d'informations

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;

// Modules backend
mod backend;
use backend::{
    db::Database,
    tor::TorManager,
    mla::MlaContext,
    entities::EntityManager,
    watch::WatchScheduler,
    social::SocialMediaManager,
    ner::NerManager,
    dedup::DeduplicationManager,
    cotations::CotationManager,
    api::GenericApiClient,
    capitalize::CapitalizationManager,
    exploit::ExploitManager,
    gerer::GererManager,
};

// Module partagé
use osint_shared::models::AppSettings;

/// Structure globale de l'application
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub tor_manager: Arc<Mutex<TorManager>>,
    pub mla_context: Arc<Mutex<MlaContext>>,
    pub settings: Arc<Mutex<AppSettings>>,
    pub entity_manager: Arc<Mutex<EntityManager>>,
}

fn main() {
    // Initialiser le logger
    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .init();
    
    log::info!("Démarrage de l'application OSINT...");
    
    // Charger les paramètres
    let settings = AppSettings::load().expect("Échec du chargement des paramètres");
    log::info!("Paramètres chargés");
    
    // Initialiser le contexte MLA
    let mla_context = MlaContext::new().expect("Échec de l'initialisation de MLA");
    log::info!("Contexte MLA initialisé");
    
    // Initialiser la base de données
    let db = Database::new(&settings).expect("Échec de l'initialisation de la base de données");
    log::info!("Base de données initialisée");
    
    // Initialiser le gestionnaire TOR
    let tor_manager = TorManager::new(&settings.tor).expect("Échec de l'initialisation de TOR");
    log::info!("Gestionnaire TOR initialisé");
    
    // Initialiser le gestionnaire d'entités
    let entity_manager = EntityManager::new(Arc::clone(&db));
    
    // Créer l'état global
    let app_state = AppState {
        db: Arc::new(Mutex::new(db)),
        tor_manager: Arc::new(Mutex::new(tor_manager)),
        mla_context: Arc::new(Mutex::new(mla_context)),
        settings: Arc::new(Mutex::new(settings)),
        entity_manager: Arc::new(Mutex::new(entity_manager)),
    };
    
    // Démarrer le planificateur de veilles en arrière-plan
    let db_clone = Arc::clone(&app_state.db);
    let tor_clone = Arc::clone(&app_state.tor_manager);
    tokio::spawn(async move {
        let mut watch_scheduler = WatchScheduler::new(
            db_clone,
            tor_clone,
        );
        watch_scheduler.start().await;
    });
    
    // Démarrer le daemon TOR si configuré
    if app_state.settings.lock().unwrap().tor.auto_start {
        let tor_manager_clone = Arc::clone(&app_state.tor_manager);
        tokio::spawn(async move {
            if let Err(e) = tor_manager_clone.lock().unwrap().start_tor() {
                log::error!("Échec du démarrage de TOR: {}", e);
            } else {
                log::info!("TOR démarré avec succès");
            }
        });
    }
    
    // Configurer Tauri
    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            // Initialiser la fenêtre principale
            let window = app.get_webview_window("main").unwrap();
            window.set_title("OSINT App - Accueil")?;
            
            // Définir la taille minimale
            window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize { width: 1024.0, height: 768.0 })))?;
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Gestion des paramètres
            backend::settings::get_settings,
            backend::settings::save_settings,
            backend::settings::get_secrets_socket_path,
            
            // Gestion TOR
            backend::tor::get_tor_status,
            backend::tor::start_tor,
            backend::tor::stop_tor,
            backend::tor::get_tor_exit_nodes,
            backend::tor::test_tor_connection,
            backend::tor::refresh_tor_exit_nodes,
            backend::tor::select_tor_exit_node,
            
            // Gestion de la base de données
            backend::db::test_db_connection,
            backend::db::get_database_path,
            
            // Import de fichiers
            backend::importer::import_file,
            backend::importer::check_file_duplicate,
            backend::importer::get_source_by_name,
            backend::importer::create_source,
            backend::importer::process_file_with_ner,
            
            // Veille internet
            backend::watch::get_watches,
            backend::watch::add_watch,
            backend::watch::update_watch,
            backend::watch::delete_watch,
            backend::watch::get_watch_results,
            backend::watch::save_watch_result,
            backend::watch::mark_result_processed,
            backend::watch::get_processed_watch_results,
            backend::watch::process_watch_results,
            backend::watch::group_results_by_event,
            
            // Planificateur de veilles
            backend::watch::scheduler::start_watch_scheduler,
            backend::watch::scheduler::stop_watch_scheduler,
            backend::watch::scheduler::run_watch_manually,
            
            // Mode découverte
            backend::discovery::discover_topic,
            backend::discovery::add_discovered_sources_to_watch,
            
            // Réseaux sociaux
            backend::social::social_search,
            backend::social::get_social_platforms,
            backend::social::save_social_account,
            
            // NER
            backend::ner::extract_entities,
            backend::ner::extract_and_match_entities,
            backend::ner::extract_relations,
            backend::ner::save_ner_result,
            backend::ner::get_entity_types,
            
            // Détection de doublons
            backend::dedup::check_duplicates,
            backend::dedup::merge_entities,
            backend::dedup::cleanup_duplicates,
            backend::dedup::get_deduplication_strategies,
            
            // Cotations OTAN
            backend::cotations::propose_cotation,
            backend::cotations::get_entity_cotation,
            backend::cotations::update_entity_cotation,
            backend::cotations::get_reliability_codes,
            backend::cotations::get_credibility_codes,
            backend::cotations::validate_cotation,
            backend::cotations::parse_cotation,
            backend::cotations::get_cotation_label,
            backend::cotations::get_average_cotation,
            
            // API générique
            backend::api::api_request,
            backend::api::mistral_chat,
            backend::api::mistral_embeddings,
            backend::api::check_api_health,
            
            // Entités
            backend::entities::create_entity,
            backend::entities::create_individual,
            backend::entities::create_organization,
            backend::entities::create_event,
            backend::entities::create_location,
            backend::entities::create_object,
            backend::entities::create_source,
            backend::entities::get_entity,
            backend::entities::search_entities,
            backend::entities::link_entities,
            backend::entities::get_entity_links,
            backend::entities::enrich_entity,
            backend::entities::get_individual_types,
            backend::entities::get_location_types,
            backend::entities::get_organization_types,
            backend::entities::get_object_types,
            backend::entities::update_entity,
            backend::entities::delete_entity,
            
            // Capitalisation
            backend::capitalize::capitalize_watch_result,
            backend::capitalize::capitalize_event_group,
            backend::capitalize::validate_capitalized_info,
            backend::capitalize::get_unvalidated_capitalized_infos,
            backend::capitalize::get_capitalized_infos_by_watch,
            backend::capitalize::auto_capitalize_watch_result,
            
            // Exploitation
            backend::exploit::generate_relational_graph,
            backend::exploit::generate_timeline,
            backend::exploit::generate_map_points,
            backend::exploit::get_tile_providers,
            backend::exploit::get_tile_provider_info,
            backend::exploit::get_acled_data,
            
            // Gestion
            backend::gerer::detect_duplicates,
            backend::gerer::get_duplicate_review_queue,
            backend::gerer::process_duplicate_review_item,
            backend::gerer::auto_clean_duplicates,
            backend::gerer::get_database_stats,
            backend::gerer::backup_database,
            backend::gerer::restore_database,
            backend::gerer::optimize_database,
            backend::gerer::check_database_integrity,
            backend::gerer::export_data_json,
            backend::gerer::import_data_json,
            backend::gerer::get_duplicate_review_statuses,
            backend::gerer::get_duplicate_actions,
        ])
        .run(tauri::generate_context!())
        .expect("Échec de l'exécution de l'application");
}
