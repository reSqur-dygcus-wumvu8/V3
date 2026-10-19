use crate::AppState;
// Module de gestion des réseaux sociaux
// Gère la collecte et le traitement des données depuis Twitter/X, Discord, Telegram, TikTok, VKontakte

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use log::{info, error, warn};
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::backend::{db::Database, tor::TorClient};
use super::watch::{WatchResult, ApiClient};

/// Type de réseau social
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SocialPlatform {
    Twitter,
    Discord,
    Telegram,
    TikTok,
    VKontakte,
}

impl SocialPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            SocialPlatform::Twitter => "twitter",
            SocialPlatform::Discord => "discord",
            SocialPlatform::Telegram => "telegram",
            SocialPlatform::TikTok => "tiktok",
            SocialPlatform::VKontakte => "vkontakte",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "twitter" | "twitter/x" | "x" => Some(SocialPlatform::Twitter),
            "discord" => Some(SocialPlatform::Discord),
            "telegram" => Some(SocialPlatform::Telegram),
            "tiktok" => Some(SocialPlatform::TikTok),
            "vkontakte" | "vk" | "вконтакте" => Some(SocialPlatform::VKontakte),
            _ => None,
        }
    }
}

/// Configuration pour un compte de réseau social
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialAccount {
    pub id: Option<i64>,
    pub platform: SocialPlatform,
    pub account_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub url: Option<String>,
    pub is_verified: bool,
    pub follower_count: Option<u64>,
    pub description: Option<String>,
    pub last_fetched: Option<String>,
}

/// Résultat d'une recherche sur les réseaux sociaux
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialSearchResult {
    pub platform: SocialPlatform,
    pub post_id: String,
    pub post_url: String,
    pub author: SocialAccount,
    pub content: String,
    pub published_at: Option<String>,
    pub engagement: SocialEngagement,
    pub metadata: serde_json::Value,
}

/// Métriques d'engagement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialEngagement {
    pub likes: Option<u64>,
    pub reposts: Option<u64>,
    pub replies: Option<u64>,
    pub views: Option<u64>,
}

/// Gestionnaire des réseaux sociaux
pub struct SocialMediaManager {
    db: Arc<Mutex<Database>>,
    tor_client: TorClient,
    api_client: Option<ApiClient>,
}

impl SocialMediaManager {
    pub fn new(db: Arc<Mutex<Database>>, tor_client: TorClient) -> Self {
        Self {
            db,
            tor_client,
            api_client: None,
        }
    }

    pub fn with_api_client(mut self, api_client: ApiClient) -> Self {
        self.api_client = Some(api_client);
        self
    }

    /// Recherche sur un réseau social
    pub async fn search(
        &self,
        platform: SocialPlatform,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SocialSearchResult>, String> {
        match platform {
            SocialPlatform::Twitter => self.search_twitter(query, limit).await,
            SocialPlatform::Discord => self.search_discord(query, limit).await,
            SocialPlatform::Telegram => self.search_telegram(query, limit).await,
            SocialPlatform::TikTok => self.search_tiktok(query, limit).await,
            SocialPlatform::VKontakte => self.search_vkontakte(query, limit).await,
        }
    }

    /// Recherche sur Twitter/X
    pub async fn search_twitter(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SocialSearchResult>, String> {
        let api_client = self.api_client.as_ref()
            .ok_or("ApiClient non initialisé")?;

        // Utiliser l'API Twitter v2
        let url = "https://api.twitter.com/2/tweets/search/recent";
        let max_results = limit.unwrap_or(10).min(100);

        let params = [
            ("query", query),
            ("max_results", max_results.to_string().as_str()),
            ("tweet.fields", "created_at,author_id,public_metrics,text"),
            ("user.fields", "username,name,verified,description,public_metrics"),
        ];

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let mut request = client.get(url);
        for (key, value) in params {
            request = request.query(&[(key, value)]);
        }

        let response = request
            .header("Authorization", format!("Bearer {}", api_client.mistral_api_key.as_ref().unwrap_or(&"".to_string())))
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP Twitter: {}", e))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Twitter API erreur: {} - {}", response.status(), body));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        self.parse_twitter_results(&json)
    }

    /// Parse les résultats Twitter
    fn parse_twitter_results(&self, json: &serde_json::Value) -> Result<Vec<SocialSearchResult>, String> {
        let mut results = Vec::new();

        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
            if let Some(includes) = json.get("includes").and_then(|i| i.get("users")) {
                if let Some(users) = includes.as_array() {
                    for tweet in data {
                        let text = tweet.get("text").and_then(|t| t.as_str()).unwrap_or_default();
                        let tweet_id = tweet.get("id").and_then(|id| id.as_str()).unwrap_or_default();
                        let created_at = tweet.get("created_at").and_then(|c| c.as_str()).map(|s| s.to_string());
                        
                        let author_id = tweet.get("author_id").and_then(|a| a.as_str()).unwrap_or_default();
                        
                        // Trouver l'utilisateur
                        let user = users.iter()
                            .find(|u| u.get("id").and_then(|id| id.as_str()) == Some(author_id))
                            .cloned();

                        let (username, display_name, verified, description) = if let Some(user) = user {
                            (
                                user.get("username").and_then(|u| u.as_str()).unwrap_or_default(),
                                user.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()),
                                user.get("verified").and_then(|v| v.as_bool()).unwrap_or(false),
                                user.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                            )
                        } else {
                            (String::new(), None, false, None)
                        };

                        // Métriques
                        let metrics = tweet.get("public_metrics").and_then(|m| m.as_object()).cloned();
                        let engagement = SocialEngagement {
                            likes: metrics.as_ref().and_then(|m| m.get("like_count").and_then(|l| l.as_u64())),
                            reposts: metrics.as_ref().and_then(|m| m.get("retweet_count").and_then(|r| r.as_u64())),
                            replies: metrics.as_ref().and_then(|m| m.get("reply_count").and_then(|r| r.as_u64())),
                            views: metrics.as_ref().and_then(|m| m.get("impression_count").and_then(|v| v.as_u64())),
                        };

                        let post_url = format!("https://twitter.com/{}/status/{}", username, tweet_id);

                        results.push(SocialSearchResult {
                            platform: SocialPlatform::Twitter,
                            post_id: tweet_id,
                            post_url,
                            author: SocialAccount {
                                id: None,
                                platform: SocialPlatform::Twitter,
                                account_id: author_id,
                                username,
                                display_name,
                                url: Some(format!("https://twitter.com/{}", username)),
                                is_verified: verified,
                                follower_count: user.as_ref().and_then(|u| u.get("public_metrics").and_then(|m| m.get("followers_count").and_then(|f| f.as_u64()))),
                                description,
                                last_fetched: None,
                            },
                            content: text,
                            published_at: created_at,
                            engagement,
                            metadata: json!({
                                "platform": "twitter",
                                "tweet_id": tweet_id,
                                "author_id": author_id
                            }),
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// Recherche sur Discord
    pub async fn search_discord(
        &self,
        query: &str,
        _limit: Option<usize>,
    ) -> Result<Vec<SocialSearchResult>, String> {
        // Discord n'a pas d'API publique de recherche
        // On peut utiliser des webhooks ou des bots, mais pas de recherche directe
        warn!("La recherche Discord n'est pas disponible via l'API publique");
        Ok(Vec::new())
    }

    /// Recherche sur Telegram
    pub async fn search_telegram(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SocialSearchResult>, String> {
        // Telegram n'a pas d'API publique de recherche
        // On peut utiliser l'API Bot pour rechercher dans des canaux spécifiques
        warn!("La recherche Telegram nécessite l'API Bot et des canaux spécifiques");
        Ok(Vec::new())
    }

    /// Recherche sur TikTok
    pub async fn search_tiktok(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SocialSearchResult>, String> {
        // TikTok a une API non officielle
        let client = self.tor_client.client.clone();
        let encoded_query = urlencoding::encode(query);
        let url = format!("https://www.tiktok.com/search?keyword={}", encoded_query);

        let response = client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP TikTok: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("TikTok a retourné un code HTTP {}", response.status()));
        }

        let body = response.text().await.map_err(|e| e.to_string())?;

        // TikTok utilise beaucoup de JavaScript, le scraping est complexe
        // Pour l'instant, on retourne un message d'avertissement
        warn!("Le scraping TikTok nécessite une approche plus sophistiquée (headless browser)");
        Ok(Vec::new())
    }

    /// Recherche sur VKontakte
    pub async fn search_vkontakte(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SocialSearchResult>, String> {
        let client = self.tor_client.client.clone();
        let encoded_query = urlencoding::encode(query);
        let url = format!("https://vk.com/search?c%5Bsection%5D=auto&c%5Bq%5D={}", encoded_query);

        let response = client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP VKontakte: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("VKontakte a retourné un code HTTP {}", response.status()));
        }

        let body = response.text().await.map_err(|e| e.to_string())?;

        // Parser les résultats VKontakte
        self.parse_vkontakte_results(&body)
    }

    /// Parse les résultats VKontakte
    fn parse_vkontakte_results(&self, html: &str) -> Result<Vec<SocialSearchResult>, String> {
        // VKontakte nécessite un parsing HTML complexe
        // Pour l'instant, on retourne un message d'avertissement
        warn!("Le parsing VKontakte nécessite une implémentation plus sophistiquée");
        Ok(Vec::new())
    }

    /// Convertit les résultats sociaux en WatchResult pour la capitalisation
    pub fn convert_to_watch_results(
        &self,
        results: Vec<SocialSearchResult>,
        watch_id: i64,
    ) -> Vec<WatchResult> {
        results.into_iter().map(|social_result| {
            WatchResult {
                id: None,
                watch_id,
                title: social_result.content.chars().take(100).collect::<String>(),
                url: social_result.post_url,
                content: social_result.content,
                translated_content: None,
                source_name: format!("{}", social_result.platform.as_str()),
                published_at: social_result.published_at,
                fetched_at: None,
                is_processed: false,
                metadata: Some(json!({
                    "platform": social_result.platform.as_str(),
                    "post_id": social_result.post_id,
                    "author": social_result.author.username,
                    "engagement": {
                        "likes": social_result.engagement.likes,
                        "reposts": social_result.engagement.reposts,
                        "replies": social_result.engagement.replies,
                        "views": social_result.engagement.views,
                    }
                })),
                entities: None,
            }
        }).collect()
    }

    /// Sauvegarde un compte de réseau social dans la base
    pub async fn save_social_account(&self, account: SocialAccount) -> Result<i64, String> {
        let db = self.db.lock().await;
        
        let platform_str = account.platform.as_str();
        let now = chrono::Utc::now().to_rfc3339();

        let account_id = db.conn.execute(
            "INSERT INTO social_accounts (platform, account_id, username, display_name, url, is_verified, follower_count, description, last_fetched) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                platform_str,
                account.account_id,
                account.username,
                account.display_name.unwrap_or_default(),
                account.url.unwrap_or_default(),
                account.is_verified,
                account.follower_count.unwrap_or(0) as i64,
                account.description.unwrap_or_default(),
                now,
            ),
        ).map_err(|e| e.to_string())?;

        Ok(account_id as i64)
    }

    /// Récupère un compte de réseau social par son ID
    pub async fn get_social_account(
        &self,
        platform: SocialPlatform,
        account_id: &str,
    ) -> Result<Option<SocialAccount>, String> {
        let db = self.db.lock().await;
        let platform_str = platform.as_str();

        let mut stmt = db.conn.prepare(
            "SELECT id, platform, account_id, username, display_name, url, is_verified, follower_count, description, last_fetched 
             FROM social_accounts 
             WHERE platform = ? AND account_id = ?"
        ).map_err(|e| e.to_string())?;

        let mut rows = stmt.query([platform_str, account_id])
            .map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(SocialAccount {
                id: row.get(0).ok(),
                platform: SocialPlatform::from_str(&row.get::<_, String>(1).unwrap_or_default())
                    .unwrap_or(SocialPlatform::Twitter),
                account_id: row.get(2).unwrap_or_default(),
                username: row.get(3).unwrap_or_default(),
                display_name: row.get(4).ok(),
                url: row.get(5).ok(),
                is_verified: row.get(6).unwrap_or(false),
                follower_count: row.get(7).ok(),
                description: row.get(8).ok(),
                last_fetched: row.get(9).ok(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Lie un compte de réseau social à une entité
    pub async fn link_social_account_to_entity(
        &self,
        account_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<(), String> {
        let db = self.db.lock().await;

        db.conn.execute(
            "INSERT INTO entity_social_accounts (entity_type, entity_id, account_id) 
             VALUES (?, ?, ?)",
            (entity_type, entity_id, account_id),
        ).map_err(|e| e.to_string())?;

        Ok(())
    }
}

/// Commandes Tauri pour les réseaux sociaux


#[allow(dead_code)]
pub async fn social_search(
    
    
        
        
            
        state: &crate::AppState,
    platform: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SocialSearchResult>, String> {
    let tor_client = state.tor_manager.lock().await;
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );

    let social_manager = SocialMediaManager::new(
        Arc::clone(&state.db),
        tor_client.clone(),
    ).with_api_client(api_client);

    let platform_enum = SocialPlatform::from_str(&platform)
        .ok_or_else(|| format!("Plateforme inconnue: {}", platform))?;

    social_manager.search(platform_enum, &query, limit).await
}


#[allow(dead_code)]
pub async fn get_social_platforms() -> Result<Vec<String>, String> {
    let platforms = vec![
        SocialPlatform::Twitter.as_str().to_string(),
        SocialPlatform::Discord.as_str().to_string(),
        SocialPlatform::Telegram.as_str().to_string(),
        SocialPlatform::TikTok.as_str().to_string(),
        SocialPlatform::VKontakte.as_str().to_string(),
    ];
    Ok(platforms)
}


#[allow(dead_code)]
pub async fn save_social_account(
    
    
        
        
            
        state: &crate::AppState,
    platform: String,
    account_id: String,
    username: String,
    display_name: Option<String>,
    url: Option<String>,
    is_verified: bool,
    follower_count: Option<u64>,
    description: Option<String>,
) -> Result<i64, String> {
    let platform_enum = SocialPlatform::from_str(&platform)
        .ok_or_else(|| format!("Plateforme inconnue: {}", platform))?;

    let tor_client = state.tor_manager.lock().await;
    let api_client = ApiClient::new(
        tor_client.create_tor_client()?,
        state.settings.lock().await.api.mistral_key.clone(),
    );

    let social_manager = SocialMediaManager::new(
        Arc::clone(&state.db),
        tor_client.clone(),
    ).with_api_client(api_client);

    let account = SocialAccount {
        id: None,
        platform: platform_enum,
        account_id,
        username,
        display_name,
        url,
        is_verified,
        follower_count,
        description,
        last_fetched: None,
    };

    social_manager.save_social_account(account).await
}
