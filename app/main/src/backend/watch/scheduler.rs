use crate::AppState;
// Planificateur de tâches pour les veilles
// Gère l'exécution périodique des veilles et le traitement des résultats

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time;
use anyhow::{Result, Context};
use log::{info, error, warn};

use crate::backend::{db::Database, tor::TorClient};
use super::{Watch, WatchResult, WatchManager, ApiClient, ScraperFactory, scrapers::Scraper};

/// Planificateur de veilles
pub struct WatchScheduler {
    db: Arc<Mutex<Database>>,
    tor_client: Arc<Mutex<TorClient>>,
    api_client: Option<ApiClient>,
    scraper_factory: Option<ScraperFactory>,
    is_running: bool,
    last_run: Option<Instant>,
}

impl WatchScheduler {
    /// Crée un nouveau planificateur
    pub fn new(db: Arc<Mutex<Database>>, tor_client: Arc<Mutex<TorClient>>) -> Self {
        Self {
            db,
            tor_client,
            api_client: None,
            scraper_factory: None,
            is_running: false,
            last_run: None,
        }
    }

    /// Initialise le planificateur avec les dépendances nécessaires
    pub fn with_dependencies(
        mut self,
        api_client: ApiClient,
        scraper_factory: ScraperFactory,
    ) -> Self {
        self.api_client = Some(api_client);
        self.scraper_factory = Some(scraper_factory);
        self
    }

    /// Démarre le planificateur
    pub async fn start(&mut self) {
        if self.is_running {
            warn!("Le planificateur de veilles est déjà en cours d'exécution");
            return;
        }

        info!("Démarrage du planificateur de veilles...");
        self.is_running = true;

        loop {
            if !self.is_running {
                break;
            }

            let start_time = Instant::now();
            info!("Début d'un cycle de veille à {:?}", start_time);

            // Exécuter toutes les veilles actives
            if let Err(e) = self.run_active_watches().await {
                error!("Erreur lors de l'exécution des veilles: {}", e);
            }

            self.last_run = Some(Instant::now());

            // Calculer le temps d'attente jusqu'au prochain cycle
            let sleep_duration = self.calculate_sleep_duration().await;
            info!("Prochain cycle dans {:?}", sleep_duration);

            // Attendre avant le prochain cycle
            time::sleep(sleep_duration).await;
        }

        info!("Planificateur de veilles arrêté");
    }

    /// Arrête le planificateur
    pub fn stop(&mut self) {
        self.is_running = false;
        info!("Arrêt du planificateur de veilles");
    }

    /// Exécute toutes les veilles actives
    async fn run_active_watches(&self) -> Result<()> {
        let db = self.db.lock().await;
        let watches = db.get_active_watches()?;

        info!("Exécution de {} veilles actives", watches.len());

        for watch in watches {
            if let Err(e) = self.run_watch(&watch).await {
                error!("Erreur lors de l'exécution de la veille '{}': {}", watch.name, e);
            }
        }

        Ok(())
    }

    /// Exécute une veille spécifique
    async fn run_watch(&self, watch: &Watch) -> Result<()> {
        info!("Exécution de la veille '{}' pour la requête: {}", watch.name, watch.query);

        // Vérifier que les dépendances sont initialisées
        let api_client = self.api_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ApiClient non initialisé"))?;
        let scraper_factory = self.scraper_factory.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ScraperFactory non initialisé"))?;

        let tor_client = self.tor_client.lock().await;

        // Sélectionner le nœud TOR adapté
        let target_country = self.get_target_country(&watch.sources);
        if let Some(node) = tor_client.select_exit_node(target_country.as_deref()) {
            info!("Utilisation du nœud TOR: {} ({})", node.nickname, node.country);
            // Note: Pour l'instant, on ne peut pas changer de nœud dynamiquement
            // sans redémarrer TOR. Cela nécessiterait une intégration plus poussée.
        }

        // Exécuter le scraping pour chaque source
        let mut all_results = Vec::new();

        for source in &watch.sources {
            let scraper = scraper_factory.create_scraper(source);
            
            match scraper.scrape(&watch.query, &watch.language).await {
                Ok(results) => {
                    info!("Récupérés {} résultats depuis {}", results.len(), source);
                    
                    // Traduire les résultats en français
                    let translated_results = self.translate_results(results, &watch.language, api_client).await?;
                    
                    // Sauvegarder les résultats dans la base
                    self.save_results(&translated_results, watch.id.unwrap_or(0)).await?;
                    
                    all_results.extend(translated_results);
                }
                Err(e) => {
                    error!("Erreur lors du scraping de {}: {}", source, e);
                }
            }
        }

        // Traiter les résultats (NER, cotation, etc.)
        self.process_results(all_results, watch.id.unwrap_or(0), api_client).await?;

        Ok(())
    }

    /// Détermine le pays cible pour la sélection du nœud TOR
    fn get_target_country(&self, sources: &[String]) -> Option<String> {
        // Priorité aux sources qui nécessitent un pays spécifique
        for source in sources {
            match source.as_str() {
                "google_news" => return Some("US".to_string()),
                "qwant" => return Some("FR".to_string()),
                "yandex" => return Some("RU".to_string()),
                "twitter" => return Some("US".to_string()),
                "telegram" => return Some("DE".to_string()), // Telegram a des serveurs en Allemagne
                _ => {}
            }
        }
        None
    }

    /// Traduit les résultats en français
    async fn translate_results(
        &self,
        results: Vec<WatchResult>,
        source_language: &str,
        api_client: &ApiClient,
    ) -> Result<Vec<WatchResult>, String> {
        let mut translated_results = Vec::new();

        for result in results {
            // Ne pas traduire si la source est déjà en français
            if source_language == "fr" {
                translated_results.push(result);
                continue;
            }

            // Traduire le contenu
            let translated_content = api_client.translate(&result.content, "fr").await?;

            let mut translated_result = result;
            translated_result.translated_content = Some(translated_content);
            translated_results.push(translated_result);
        }

        Ok(translated_results)
    }

    /// Sauvegarde les résultats dans la base
    async fn save_results(&self, results: Vec<WatchResult>, watch_id: i64) -> Result<()> {
        let db = self.db.lock().await;

        for result in results {
            let mut result_with_watch_id = result;
            result_with_watch_id.watch_id = watch_id;
            
            // Sauvegarder le résultat
            db.save_watch_result(result_with_watch_id)?;
        }

        Ok(())
    }

    /// Traite les résultats (NER, cotation, etc.)
    async fn process_results(
        &self,
        results: Vec<WatchResult>,
        watch_id: i64,
        api_client: &ApiClient,
    ) -> Result<()> {
        // Pour l'instant, on marque simplement les résultats comme traités
        // Le traitement complet (NER, cotation) sera implémenté dans le module processor
        let db = self.db.lock().await;

        for result in results {
            if let Some(id) = result.id {
                db.mark_watch_result_processed(id)?;
            }
        }

        Ok(())
    }

    /// Calcule la durée d'attente jusqu'au prochain cycle
    async fn calculate_sleep_duration(&self) -> Duration {
        // Récupérer toutes les veilles actives pour trouver la fréquence minimale
        let db = self.db.lock().await;
        
        match db.get_active_watches() {
            Ok(watches) => {
                if watches.is_empty() {
                    // Si aucune veille active, attendre 1 heure
                    return Duration::from_secs(3600);
                }

                // Trouver la fréquence minimale
                let min_frequency = watches.iter()
                    .map(|w| w.frequency_minutes)
                    .min()
                    .unwrap_or(60);

                // Convertir en secondes et ajouter une marge de 10 secondes
                Duration::from_secs((min_frequency as u64 * 60) + 10)
            }
            Err(e) => {
                error!("Erreur lors de la récupération des veilles: {}", e);
                // En cas d'erreur, attendre 5 minutes
                Duration::from_secs(300)
            }
        }
    }

    /// Exécute une veille manuellement (pour les tests)
    pub async fn run_watch_manually(&self, watch_id: i64) -> Result<Vec<WatchResult>, String> {
        let db = self.db.lock().await;
        let watches = db.get_active_watches()?;

        let watch = watches.into_iter()
            .find(|w| w.id == Some(watch_id))
            .ok_or_else(|| anyhow::anyhow!("Veille non trouvée"))?;

        let api_client = self.api_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ApiClient non initialisé"))?;
        let scraper_factory = self.scraper_factory.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ScraperFactory non initialisé"))?;

        let mut all_results = Vec::new();

        for source in &watch.sources {
            let scraper = scraper_factory.create_scraper(source);
            
            match scraper.scrape(&watch.query, &watch.language).await {
                Ok(results) => {
                    let translated_results = self.translate_results(results, &watch.language, api_client).await?;
                    all_results.extend(translated_results);
                }
                Err(e) => {
                    error!("Erreur lors du scraping de {}: {}", source, e);
                }
            }
        }

        Ok(all_results)
    }
}

/// Commandes Tauri pour le planificateur


#[allow(dead_code)]
pub async fn start_watch_scheduler(
    
        
        
            
        state: &crate::AppState) -> Result<(), String> {
    let mut watch_scheduler = WatchScheduler::new(
        Arc::clone(&state.db),
        Arc::clone(&state.tor_manager),
    );

    // Initialiser les dépendances
    let tor_client = state.tor_manager.lock().await;
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );
    let scraper_factory = ScraperFactory::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );

    watch_scheduler = watch_scheduler.with_dependencies(api_client, scraper_factory);

    // Démarrer le planificateur en arrière-plan
    tokio::spawn(async move {
        watch_scheduler.start().await;
    });

    Ok(())
}


#[allow(dead_code)]
pub async fn stop_watch_scheduler(
    
        
        
            
        state: &crate::AppState) -> Result<(), String> {
    // Pour l'instant, on ne peut pas arrêter le planificateur depuis l'extérieur
    // car il est lancé dans un tokio::spawn séparé.
    // Cela nécessiterait une gestion d'état globale.
    Ok(())
}


#[allow(dead_code)]
pub async fn run_watch_manually(
    
    
        
        
            
        state: &crate::AppState,
    watch_id: i64,
) -> Result<Vec<WatchResult>, String> {
    let mut watch_scheduler = WatchScheduler::new(
        Arc::clone(&state.db),
        Arc::clone(&state.tor_manager),
    );

    // Initialiser les dépendances
    let tor_client = state.tor_manager.lock().await;
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );
    let scraper_factory = ScraperFactory::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );

    watch_scheduler = watch_scheduler.with_dependencies(api_client, scraper_factory);

    // Exécuter la veille manuellement
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(watch_scheduler.run_watch_manually(watch_id))
}
