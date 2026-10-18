//! Module API générique pour les appels externes
//! Gère les requêtes HTTP via TOR avec gestion des erreurs et retry

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use log::{info, error, warn, debug};
use reqwest::{Client, Response, StatusCode};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};

use crate::backend::tor::TorClient;

/// Configuration pour une requête API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequestConfig {
    pub url: String,
    pub method: String,
    pub headers: Option<Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout: Option<u64>,
    pub max_retries: Option<usize>,
    pub retry_delay: Option<u64>,
}

impl Default for ApiRequestConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: "GET".to_string(),
            headers: None,
            body: None,
            timeout: Some(30),
            max_retries: Some(3),
            retry_delay: Some(1000), // 1 seconde
        }
    }
}

/// Résultat d'une requête API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status_code: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub is_success: bool,
    pub error: Option<String>,
}

/// Client API générique
#[derive(Clone)]
pub struct GenericApiClient {
    pub tor_client: TorClient,
    pub default_timeout: Duration,
    pub default_retries: usize,
    pub default_retry_delay: Duration,
}

impl GenericApiClient {
    pub fn new(tor_client: TorClient) -> Self {
        Self {
            tor_client,
            default_timeout: Duration::from_secs(30),
            default_retries: 3,
            default_retry_delay: Duration::from_millis(1000),
        }
    }

    /// Exécute une requête API générique
    pub async fn request(
        &self,
        config: ApiRequestConfig,
    ) -> Result<ApiResponse, String> {
        let method = config.method.to_uppercase();
        let url = config.url.clone();
        
        debug!("Requête API: {} {}", method, url);

        let timeout = config.timeout.map(Duration::from_secs).unwrap_or(self.default_timeout);
        let max_retries = config.max_retries.unwrap_or(self.default_retries);
        let retry_delay = config.retry_delay.map(Duration::from_millis).unwrap_or(self.default_retry_delay);

        let mut last_error: Option<String> = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                warn!("Nouvelle tentative ({}/{}) pour {} {}", attempt, max_retries, method, url);
                tokio::time::sleep(retry_delay).await;
            }

            match self.execute_request(&method, &url, &config, timeout).await {
                Ok(response) => {
                    // Vérifier si la réponse est un succès
                    if response.is_success {
                        return Ok(response);
                    }

                    // Pour les erreurs 429 (Too Many Requests), on peut retry
                    if response.status_code == 429 && attempt < max_retries {
                        last_error = Some(format!("Rate limit (429) pour {} {}", method, url));
                        continue;
                    }

                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e.clone());
                    
                    // Retry pour les erreurs réseau
                    if attempt < max_retries {
                        continue;
                    }
                    
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "Erreur inconnue".to_string()))
    }

    /// Exécute une requête individuelle
    async fn execute_request(
        &self,
        method: &str,
        url: &str,
        config: &ApiRequestConfig,
        timeout: Duration,
    ) -> Result<ApiResponse, String> {
        let client = self.tor_client.client.clone();

        let mut request_builder = match method {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            "HEAD" => client.head(url),
            _ => return Err(format!("Méthode HTTP non supportée: {}", method)),
        };

        // Ajouter les headers
        if let Some(headers) = &config.headers {
            for (key, value) in headers {
                request_builder = request_builder.header(key, value);
            }
        }

        // Ajouter le timeout
        request_builder = request_builder.timeout(timeout);

        // Ajouter le body
        if let Some(body) = &config.body {
            request_builder = request_builder.body(body.clone());
        }

        // Exécuter la requête
        let response = request_builder.send().await
            .map_err(|e| format!("Erreur HTTP: {}", e))?;

        // Lire le body
        let body = response.text().await.map_err(|e| e.to_string())?;

        // Collecter les headers
        let headers: Vec<(String, String)> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let status_code = response.status().as_u16();
        let is_success = response.status().is_success();

        Ok(ApiResponse {
            status_code,
            body,
            headers,
            is_success,
            error: if is_success { None } else { Some(format!("HTTP {}", status_code)) },
        })
    }

    /// Requête GET
    pub async fn get(
        &self,
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<ApiResponse, String> {
        let config = ApiRequestConfig {
            url: url.to_string(),
            method: "GET".to_string(),
            headers,
            ..Default::default()
        };

        self.request(config).await
    }

    /// Requête POST
    pub async fn post(
        &self,
        url: &str,
        body: Option<String>,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<ApiResponse, String> {
        let config = ApiRequestConfig {
            url: url.to_string(),
            method: "POST".to_string(),
            body,
            headers,
            ..Default::default()
        };

        self.request(config).await
    }

    /// Requête POST avec JSON
    pub async fn post_json(
        &self,
        url: &str,
        json_body: &Value,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<ApiResponse, String> {
        let body = serde_json::to_string(json_body).map_err(|e| e.to_string())?;
        
        let mut headers_with_content_type = headers.unwrap_or_default();
        headers_with_content_type.push(("Content-Type".to_string(), "application/json".to_string()));

        self.post(url, Some(body), Some(headers_with_content_type)).await
    }

    /// Requête PUT
    pub async fn put(
        &self,
        url: &str,
        body: Option<String>,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<ApiResponse, String> {
        let config = ApiRequestConfig {
            url: url.to_string(),
            method: "PUT".to_string(),
            body,
            headers,
            ..Default::default()
        };

        self.request(config).await
    }

    /// Requête DELETE
    pub async fn delete(
        &self,
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<ApiResponse, String> {
        let config = ApiRequestConfig {
            url: url.to_string(),
            method: "DELETE".to_string(),
            headers,
            ..Default::default()
        };

        self.request(config).await
    }

    /// Télécharge un fichier
    pub async fn download_file(
        &self,
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<Vec<u8>, String> {
        let config = ApiRequestConfig {
            url: url.to_string(),
            method: "GET".to_string(),
            headers,
            timeout: Some(60), // 60 secondes pour les téléchargements
            ..Default::default()
        };

        let response = self.request(config).await?;

        if !response.is_success {
            return Err(format!("Échec du téléchargement: HTTP {}", response.status_code));
        }

        // Convertir le body en bytes
        Ok(response.body.into_bytes())
    }

    /// Vérifie si une URL est accessible
    pub async fn check_url_accessible(
        &self,
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<bool, String> {
        let config = ApiRequestConfig {
            url: url.to_string(),
            method: "HEAD".to_string(),
            headers,
            timeout: Some(10),
            max_retries: Some(1),
            ..Default::default()
        };

        let response = self.request(config).await?;
        Ok(response.is_success)
    }

    /// Récupère les headers d'une URL
    pub async fn get_headers(
        &self,
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<Vec<(String, String)>, String> {
        let config = ApiRequestConfig {
            url: url.to_string(),
            method: "HEAD".to_string(),
            headers,
            timeout: Some(10),
            ..Default::default()
        };

        let response = self.request(config).await?;
        Ok(response.headers)
    }
}

/// Client API pour Mistral
#[derive(Clone)]
pub struct MistralApiClient {
    pub api_client: GenericApiClient,
    pub api_key: Option<String>,
    pub endpoint: String,
}

impl MistralApiClient {
    pub fn new(tor_client: TorClient, api_key: Option<String>) -> Self {
        Self {
            api_client: GenericApiClient::new(tor_client),
            api_key,
            endpoint: "https://api.mistral.ai/v1".to_string(),
        }
    }

    /// Appelle le endpoint de chat de Mistral
    pub async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<MistralMessage>,
        max_tokens: Option<u32>,
        temperature: Option<f64>,
        top_p: Option<f64>,
        stream: Option<bool>,
    ) -> Result<MistralResponse, String> {
        if self.api_key.is_none() {
            return Err("Clé API Mistral non configurée".to_string());
        }

        let url = format!("{}/chat/completions", self.endpoint);

        let body = json!({
            "model": model,
            "messages": messages.iter().map(|m| json!({
                "role": m.role,
                "content": m.content
            })).collect::<Vec<_>>(),
            "max_tokens": max_tokens.unwrap_or(1000),
            "temperature": temperature.unwrap_or(0.7),
            "top_p": top_p.unwrap_or(0.9),
            "stream": stream.unwrap_or(false),
        });

        let headers = vec![
            ("Authorization".to_string(), format!("Bearer {}", self.api_key.as_ref().unwrap())),
            ("Content-Type".to_string(), "application/json".to_string()),
        ];

        let response = self.api_client.post_json(&url, &body, Some(headers)).await?;

        if !response.is_success {
            return Err(format!("Erreur Mistral API: {} - {}", response.status_code, response.body));
        }

        serde_json::from_str(&response.body).map_err(|e| format!("Erreur de parsing JSON: {}", e))
    }

    /// Appelle le endpoint d'embeddings de Mistral
    pub async fn embeddings(
        &self,
        model: &str,
        input: Vec<String>,
    ) -> Result<MistralEmbeddingResponse, String> {
        if self.api_key.is_none() {
            return Err("Clé API Mistral non configurée".to_string());
        }

        let url = format!("{}/embeddings", self.endpoint);

        let body = json!({
            "model": model,
            "input": input,
        });

        let headers = vec![
            ("Authorization".to_string(), format!("Bearer {}", self.api_key.as_ref().unwrap())),
            ("Content-Type".to_string(), "application/json".to_string()),
        ];

        let response = self.api_client.post_json(&url, &body, Some(headers)).await?;

        if !response.is_success {
            return Err(format!("Erreur Mistral Embeddings API: {} - {}", response.status_code, response.body));
        }

        serde_json::from_str(&response.body).map_err(|e| format!("Erreur de parsing JSON: {}", e))
    }
}

/// Message pour Mistral
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralMessage {
    pub role: String,
    pub content: String,
}

impl MistralMessage {
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    pub fn system(content: &str) -> Self {
        Self::new("system", content)
    }

    pub fn user(content: &str) -> Self {
        Self::new("user", content)
    }

    pub fn assistant(content: &str) -> Self {
        Self::new("assistant", content)
    }
}

/// Réponse de Mistral
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralResponse {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<u64>,
    pub model: Option<String>,
    pub choices: Vec<MistralChoice>,
    pub usage: Option<MistralUsage>,
}

/// Choix de réponse de Mistral
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralChoice {
    pub index: u32,
    pub message: MistralMessage,
    pub finish_reason: Option<String>,
}

/// Utilisation des tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Réponse d'embeddings de Mistral
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralEmbeddingResponse {
    pub object: String,
    pub data: Vec<MistralEmbeddingData>,
    pub model: String,
    pub usage: MistralEmbeddingUsage,
}

/// Données d'embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralEmbeddingData {
    pub object: String,
    pub embedding: Vec<f64>,
    pub index: u32,
}

/// Utilisation des tokens pour embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralEmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

/// Commandes Tauri pour l'API générique


#[tauri::command]
pub async fn api_request(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    config: ApiRequestConfig,
) -> Result<ApiResponse, String> {
    let tor_client = state.tor_manager.lock().unwrap();
    let api_client = GenericApiClient::new(tor_client.create_tor_client()?);
    
    api_client.request(config).await
}


#[tauri::command]
pub async fn mistral_chat(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    model: String,
    messages: Vec<MistralMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f64>,
) -> Result<MistralResponse, String> {
    let tor_client = state.tor_manager.lock().unwrap();
    let api_key = state.settings.lock().unwrap().shared.api.mistral_key.clone();
    
    let mistral_client = MistralApiClient::new(tor_client.create_tor_client()?, api_key);
    
    mistral_client.chat_completion(
        &model,
        messages,
        max_tokens,
        temperature,
        None,
        None,
    ).await
}


#[tauri::command]
pub async fn mistral_embeddings(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    model: String,
    input: Vec<String>,
) -> Result<MistralEmbeddingResponse, String> {
    let tor_client = state.tor_manager.lock().unwrap();
    let api_key = state.settings.lock().unwrap().shared.api.mistral_key.clone();
    
    let mistral_client = MistralApiClient::new(tor_client.create_tor_client()?, api_key);
    
    mistral_client.embeddings(&model, input).await
}


#[tauri::command]
pub async fn check_api_health(
    
    
        
        
            
        state: tauri::State<crate::AppState>,
    url: String,
) -> Result<bool, String> {
    let tor_client = state.tor_manager.lock().unwrap();
    let api_client = GenericApiClient::new(tor_client.create_tor_client()?);
    
    api_client.check_url_accessible(&url, None).await
}
