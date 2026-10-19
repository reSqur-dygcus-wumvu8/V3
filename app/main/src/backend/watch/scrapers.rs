use crate::AppState;
// Scrapers pour les moteurs de recherche et réseaux sociaux
// Gère la récupération des résultats depuis Google, Qwant, Yandex, etc.

use std::time::Duration;
use anyhow::{Result, Context};
use reqwest::Client;
use serde_json::{json, Value};
use scraper::{Html, Selector};

use crate::backend::tor::TorClient;
use super::{Watch, WatchResult};

/// Scraper pour Google Actualités (RSS)
pub struct GoogleNewsScraper {
    client: Client,
    tor_client: TorClient,
}

impl GoogleNewsScraper {
    pub async fn new(tor_client: TorClient) -> Self {
        let client = tor_client.client.clone();
        Self { client, tor_client }
    }

    /// Récupère les résultats depuis Google Actualités
    pub async fn scrape(&self, query: &str, language: &str) -> Result<Vec<WatchResult>, String> {
        // Construire l'URL RSS de Google Actualités
        let encoded_query = urlencoding::encode(query);
        let url = format!("https://news.google.com/rss/search?q={}&hl={}", encoded_query, language);

        // Récupérer le contenu via TOR
        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP pour Google News: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Google News a retourné un code HTTP {}", response.status()));
        }

        let body = response.text().await.map_err(|e| e.to_string())?;

        // Parser le flux RSS
        self.parse_rss(&body, "Google Actualités")
    }

    /// Parse un flux RSS
    fn parse_rss(&self, rss_content: &str, source_name: &str) -> Result<Vec<WatchResult>, String> {
        // Utiliser un parser RSS simple (pour l'instant, on utilise un parsing basique)
        // En production, utiliser un crate comme `rss` ou `feed-rs`
        let document = Html::parse_document(rss_content);
        
        // Sélecteur pour les éléments <item> dans le RSS
        let item_selector = Selector::parse("item").unwrap();
        let title_selector = Selector::parse("title").unwrap();
        let link_selector = Selector::parse("link").unwrap();
        let description_selector = Selector::parse("description").unwrap();
        let pub_date_selector = Selector::parse("pubDate").unwrap();

        let mut results = Vec::new();

        for item in document.select(&item_selector) {
            let title = item.select(&title_selector)
                .next()
                .map(|t| t.text().collect::<String>())
                .unwrap_or_default();

            let url = item.select(&link_selector)
                .next()
                .map(|l| l.text().collect::<String>())
                .unwrap_or_default();

            let content = item.select(&description_selector)
                .next()
                .map(|d| d.text().collect::<String>())
                .unwrap_or_default();

            let published_at = item.select(&pub_date_selector)
                .next()
                .map(|d| d.text().collect::<String>())
                .ok();

            if !title.is_empty() && !url.is_empty() {
                results.push(WatchResult {
                    id: None,
                    watch_id: 0, // À définir lors de la sauvegarde
                    title,
                    url,
                    content,
                    translated_content: None,
                    source_name: source_name.to_string(),
                    published_at,
                    fetched_at: None,
                    is_processed: false,
                    metadata: Some(json!({"source": source_name})),
                    entities: None,
                });
            }
        }

        Ok(results)
    }
}

/// Scraper pour Qwant Actualités
pub struct QwantScraper {
    client: Client,
    tor_client: TorClient,
}

impl QwantScraper {
    pub async fn new(tor_client: TorClient) -> Self {
        let client = tor_client.client.clone();
        Self { client, tor_client }
    }

    /// Récupère les résultats depuis Qwant Actualités
    pub async fn scrape(&self, query: &str, language: &str) -> Result<Vec<WatchResult>, String> {
        // Qwant n'a pas d'API publique gratuite, on utilise le scraping
        let encoded_query = urlencoding::encode(query);
        let url = format!("https://www.qwant.com/?q={}&l={}", encoded_query, language);

        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP pour Qwant: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Qwant a retourné un code HTTP {}", response.status()));
        }

        let body = response.text().await.map_err(|e| e.to_string())?;

        // Parser le HTML pour extraire les résultats
        self.parse_qwant_html(&body, "Qwant Actualités")
    }

    /// Parse le HTML de Qwant
    fn parse_qwant_html(&self, html: &str, source_name: &str) -> Result<Vec<WatchResult>, String> {
        let document = Html::parse_document(html);
        
        // Sélecteurs pour Qwant (à adapter selon la structure HTML)
        // Note: Qwant change régulièrement sa structure, ces sélecteurs peuvent nécessiter des mises à jour
        let result_selector = Selector::parse("div.result").unwrap_or_else(|_| Selector::parse("article").unwrap());
        let title_selector = Selector::parse("h2, h3").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        let description_selector = Selector::parse("p").unwrap();

        let mut results = Vec::new();

        for result in document.select(&result_selector) {
            let title = result.select(&title_selector)
                .next()
                .map(|t| t.text().collect::<String>())
                .unwrap_or_default();

            let url = result.select(&link_selector)
                .next()
                .and_then(|l| l.value().attr("href"))
                .map(|u| self.normalize_url(u, "https://www.qwant.com"))
                .unwrap_or_default();

            let content = result.select(&description_selector)
                .next()
                .map(|d| d.text().collect::<String>())
                .unwrap_or_default();

            if !title.is_empty() && !url.is_empty() {
                results.push(WatchResult {
                    id: None,
                    watch_id: 0,
                    title,
                    url,
                    content,
                    translated_content: None,
                    source_name: source_name.to_string(),
                    published_at: None,
                    fetched_at: None,
                    is_processed: false,
                    metadata: Some(json!({"source": source_name})),
                    entities: None,
                });
            }
        }

        Ok(results)
    }

    /// Normalise une URL (ajoute le domaine si nécessaire)
    fn normalize_url(&self, url: &str, base: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else if url.starts_with("/") {
            format!("{}{}", base, url)
        } else {
            url.to_string()
        }
    }
}

/// Scraper pour Yandex Actualités
pub struct YandexScraper {
    client: Client,
    tor_client: TorClient,
}

impl YandexScraper {
    pub async fn new(tor_client: TorClient) -> Self {
        let client = tor_client.client.clone();
        Self { client, tor_client }
    }

    /// Récupère les résultats depuis Yandex Actualités
    pub async fn scrape(&self, query: &str, language: &str) -> Result<Vec<WatchResult>, String> {
        // Yandex n'a pas d'API publique gratuite, on utilise le scraping
        let encoded_query = urlencoding::encode(query);
        let url = format!("https://yandex.ru/news/search?text={}&lang={}", encoded_query, language);

        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP pour Yandex: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Yandex a retourné un code HTTP {}", response.status()));
        }

        let body = response.text().await.map_err(|e| e.to_string())?;

        // Parser le HTML pour extraire les résultats
        self.parse_yandex_html(&body, "Yandex Actualités")
    }

    /// Parse le HTML de Yandex
    fn parse_yandex_html(&self, html: &str, source_name: &str) -> Result<Vec<WatchResult>, String> {
        let document = Html::parse_document(html);
        
        // Sélecteurs pour Yandex (à adapter)
        let result_selector = Selector::parse("article.news-card").unwrap_or_else(|_| Selector::parse("div.news-item").unwrap());
        let title_selector = Selector::parse("h2, h3, .news-card__title").unwrap();
        let link_selector = Selector::parse("a.news-card__link, a").unwrap();
        let description_selector = Selector::parse(".news-card__text, p").unwrap();
        let date_selector = Selector::parse(".news-card__date, time").unwrap();

        let mut results = Vec::new();

        for result in document.select(&result_selector) {
            let title = result.select(&title_selector)
                .next()
                .map(|t| t.text().collect::<String>())
                .unwrap_or_default();

            let url = result.select(&link_selector)
                .next()
                .and_then(|l| l.value().attr("href"))
                .map(|u| self.normalize_url(u, "https://yandex.ru"))
                .unwrap_or_default();

            let content = result.select(&description_selector)
                .next()
                .map(|d| d.text().collect::<String>())
                .unwrap_or_default();

            let published_at = result.select(&date_selector)
                .next()
                .map(|d| d.text().collect::<String>())
                .ok();

            if !title.is_empty() && !url.is_empty() {
                results.push(WatchResult {
                    id: None,
                    watch_id: 0,
                    title,
                    url,
                    content,
                    translated_content: None,
                    source_name: source_name.to_string(),
                    published_at,
                    fetched_at: None,
                    is_processed: false,
                    metadata: Some(json!({"source": source_name})),
                    entities: None,
                });
            }
        }

        Ok(results)
    }

    /// Normalise une URL
    fn normalize_url(&self, url: &str, base: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else if url.starts_with("/") {
            format!("{}{}", base, url)
        } else {
            url.to_string()
        }
    }
}

/// Scraper pour Twitter/X
pub struct TwitterScraper {
    client: Client,
    tor_client: TorClient,
    api_key: Option<String>,
}

impl TwitterScraper {
    pub async fn new(tor_client: TorClient, api_key: Option<String>) -> Self {
        let client = tor_client.client.clone();
        Self { client, tor_client, api_key }
    }

    /// Récupère les résultats depuis Twitter/X
    pub async fn scrape(&self, query: &str, _language: &str) -> Result<Vec<WatchResult>, String> {
        // Twitter a une API officielle, mais elle nécessite une clé API
        if self.api_key.is_none() {
            return Err("Clé API Twitter non configurée".to_string());
        }

        // Utiliser l'API Twitter v2
        let api_key = self.api_key.as_ref().unwrap();
        let url = "https://api.twitter.com/2/tweets/search/recent";

        let params = [
            ("query", query),
            ("max_results", "10"),
            ("tweet.fields", "created_at,author_id,text"),
        ];

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        // Note: En production, utiliser le client TOR
        // Pour l'instant, on utilise le client standard pour l'exemple
        let mut request = client.get(url);
        
        for (key, value) in params {
            request = request.query(&[(key, value)]);
        }

        let response = request
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP pour Twitter API: {}", e))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Twitter API a retourné un code HTTP {}: {}", response.status(), body));
        }

        let json: Value = response.json().await.map_err(|e| e.to_string())?;

        // Parser la réponse Twitter
        self.parse_twitter_json(&json, "Twitter/X")
    }

    /// Parse la réponse JSON de Twitter
    fn parse_twitter_json(&self, json: &Value, source_name: &str) -> Result<Vec<WatchResult>, String> {
        let mut results = Vec::new();

        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
            for tweet in data {
                let text = tweet.get("text").and_then(|t| t.as_str()).unwrap_or_default();
                let url = format!("https://twitter.com/i/web/status/{}", tweet.get("id").and_then(|id| id.as_str()).unwrap_or_default());
                let created_at = tweet.get("created_at").and_then(|c| c.as_str()).map(|s| s.to_string());
                let author_id = tweet.get("author_id").and_then(|a| a.as_str()).unwrap_or_default();

                if !text.is_empty() {
                    results.push(WatchResult {
                        id: None,
                        watch_id: 0,
                        title: text.chars().take(100).collect::<String>(), // Tronquer pour le titre
                        url,
                        content: text,
                        translated_content: None,
                        source_name: source_name.to_string(),
                        published_at: created_at,
                        fetched_at: None,
                        is_processed: false,
                        metadata: Some(json!({
                            "source": source_name,
                            "author_id": author_id,
                            "platform": "twitter"
                        })),
                        entities: None,
                    });
                }
            }
        }

        Ok(results)
    }
}

/// Scraper pour Telegram
pub struct TelegramScraper {
    client: Client,
    tor_client: TorClient,
}

impl TelegramScraper {
    pub async fn new(tor_client: TorClient) -> Self {
        let client = tor_client.client.clone();
        Self { client, tor_client }
    }

    /// Récupère les résultats depuis Telegram
    pub async fn scrape(&self, query: &str, _language: &str) -> Result<Vec<WatchResult>, String> {
        // Telegram n'a pas d'API publique pour la recherche, on utilise le scraping
        // Note: Cela peut être bloqué par Telegram, utiliser avec prudence
        let encoded_query = urlencoding::encode(query);
        let url = format!("https://web.telegram.org/k/#/im?query={}", encoded_query);

        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP pour Telegram: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Telegram a retourné un code HTTP {}", response.status()));
        }

        let body = response.text().await.map_err(|e| e.to_string())?;

        // Telegram utilise beaucoup de JavaScript, le scraping est complexe
        // Pour l'instant, on retourne un message d'erreur
        Err("Scraping Telegram non implémenté (nécessite une approche différente)".to_string())
    }
}

/// Scraper générique pour les sites web
pub struct GenericWebScraper {
    client: Client,
    tor_client: TorClient,
}

impl GenericWebScraper {
    pub async fn new(tor_client: TorClient) -> Self {
        let client = tor_client.client.clone();
        Self { client, tor_client }
    }

    /// Scrape un site web générique
    pub async fn scrape_url(&self, url: &str, source_name: &str) -> Result<WatchResult, String> {
        let response = self.client
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP pour {}: {}", url, e))?;

        if !response.status().is_success() {
            return Err(format!("{} a retourné un code HTTP {}", url, response.status()));
        }

        let body = response.text().await.map_err(|e| e.to_string())?;

        // Extraire le titre et le contenu
        let document = Html::parse_document(&body);
        
        let title = document.select(&Selector::parse("title").unwrap())
            .next()
            .map(|t| t.text().collect::<String>())
            .unwrap_or_default();

        let content = document.select(&Selector::parse("p, article, div[role='main']").unwrap())
            .take(10) // Limiter à 10 paragraphes
            .map(|p| p.text().collect::<String>())
            .collect::<Vec<String>>()
            .join(" ");

        Ok(WatchResult {
            id: None,
            watch_id: 0,
            title,
            url: url.to_string(),
            content,
            translated_content: None,
            source_name: source_name.to_string(),
            published_at: None,
            fetched_at: None,
            is_processed: false,
            metadata: Some(json!({"source": source_name, "url": url})),
            entities: None,
        })
    }
}

/// Fabrique de scrapers
pub struct ScraperFactory {
    tor_client: TorClient,
    mistral_api_key: Option<String>,
}

impl ScraperFactory {
    pub async fn new(tor_client: TorClient, mistral_api_key: Option<String>) -> Self {
        Self {
            tor_client,
            mistral_api_key,
        }
    }

    /// Crée un scraper en fonction de la source
    pub async fn create_scraper(&self, source: &str) -> Box<dyn Scraper> {
        match source {
            "google_news" => Box::new(GoogleNewsScraper::new(self.tor_client.clone())),
            "qwant" => Box::new(QwantScraper::new(self.tor_client.clone())),
            "yandex" => Box::new(YandexScraper::new(self.tor_client.clone())),
            "twitter" => Box::new(TwitterScraper::new(self.tor_client.clone(), self.mistral_api_key.clone())),
            "telegram" => Box::new(TelegramScraper::new(self.tor_client.clone())),
            _ => Box::new(GenericWebScraper::new(self.tor_client.clone())),
        }
    }
}

/// Trait pour les scrapers
pub trait Scraper {
    /// Scrape une requête et retourne les résultats
    fn scrape(&self, query: String, language: String) -> Box<dyn std::future::Future<Output = Result<Vec<WatchResult>, String>> + Send + 'static>;
}

impl Scraper for GoogleNewsScraper {
    fn scrape(&self, query: String, language: String) -> Box<dyn std::future::Future<Output = Result<Vec<WatchResult>, String>> + Send + 'static> {
        Box::new(self.scrape(query, language))
    }
}

impl Scraper for QwantScraper {
    fn scrape(&self, query: String, language: String) -> Box<dyn std::future::Future<Output = Result<Vec<WatchResult>, String>> + Send + 'static> {
        Box::new(self.scrape(query, language))
    }
}

impl Scraper for YandexScraper {
    fn scrape(&self, query: String, language: String) -> Box<dyn std::future::Future<Output = Result<Vec<WatchResult>, String>> + Send + 'static> {
        Box::new(self.scrape(query, language))
    }
}

impl Scraper for TwitterScraper {
    fn scrape(&self, query: String, language: String) -> Box<dyn std::future::Future<Output = Result<Vec<WatchResult>, String>> + Send + 'static> {
        Box::new(self.scrape(query, language))
    }
}

impl Scraper for TelegramScraper {
    fn scrape(&self, query: String, language: String) -> Box<dyn std::future::Future<Output = Result<Vec<WatchResult>, String>> + Send + 'static> {
        Box::new(self.scrape(query, language))
    }
}
