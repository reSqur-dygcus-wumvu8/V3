use crate::AppState;
// Module de gestion de TOR
// Gère le daemon TOR, la sélection des nœuds de sortie et les requêtes proxy

use std::process::{Command, Child, Stdio};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use reqwest::{Client, Proxy};
use serde::{Serialize, Deserialize};
use anyhow::{Context, Result};
use tokio::sync::mpsc::{self, Sender, Receiver};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::backend::settings::AppSettings;

/// Structure pour gérer le daemon TOR
pub struct TorManager {
    settings: crate::backend::settings::TorSettings,
    process: Option<Child>,
    control_port: u16,
    socks_port: u16,
    exit_nodes: Vec<TorExitNode>,
    last_exit_nodes_update: Option<Instant>,
    node_cache: HashMap<String, Vec<TorExitNode>>, // Cache par pays
}

/// Informations sur un nœud de sortie TOR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorExitNode {
    pub nickname: String,
    pub fingerprint: String,
    pub ip: String,
    pub country: String,
    pub country_code: String,
    pub is_running: bool,
    pub last_seen: Option<String>,
}

/// Statut de TOR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorStatus {
    pub is_running: bool,
    pub version: Option<String>,
    pub socks_port: u16,
    pub control_port: u16,
    pub bootstrap_status: Option<String>,
    pub circuits: usize,
    pub streams: usize,
}

impl TorManager {
    /// Crée un nouveau TorManager avec les paramètres donnés
    pub async fn new(settings: &crate::backend::settings::TorSettings) -> Result<Self> {
        Ok(Self {
            settings: settings.clone(),
            process: None,
            control_port: settings.control_port,
            socks_port: settings.socks_port,
            exit_nodes: Vec::new(),
            last_exit_nodes_update: None,
            node_cache: HashMap::new(),
        })
    }
    
    /// Démarre le daemon TOR
    pub async fn start_tor(&mut self) -> Result<()> {
        if self.process.is_some() {
            return Ok(()); // Déjà démarré
        }
        
        // Déterminer le chemin vers l'exécutable TOR
        let tor_path = self.find_tor_executable()?;
        
        // Configuration du data directory
        let data_dir = self.settings.data_directory.clone()
            .unwrap_or_else(|| PathBuf::from("/tmp/tor_data"));
        
        std::fs::create_dir_all(&data_dir)?;
        
        // Construire la commande
        let mut command = Command::new(tor_path);
        command
            .arg("--SOCKSPort")
            .arg(self.socks_port.to_string())
            .arg("--ControlPort")
            .arg(self.control_port.to_string())
            .arg("--DataDirectory")
            .arg(data_dir.to_string_lossy())
            .arg("--RunAsDaemon")
            .arg("0") // Ne pas se détacher
            .arg("--Log")
            .arg("notice")
            .arg("stdout")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        // Démarrer le processus
        let mut child = command.spawn()
            .with_context(|| "Échec du démarrage de TOR")?;
        
        // Lire la sortie pour vérifier que TOR démarre correctement
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);
        
        // Attendre que TOR soit prêt (bootstrap à 100%)
        let mut bootstrap_complete = false;
        for line in stdout_reader.lines() {
            if let Ok(line) = line {
                if line.contains("Bootstrapped 100%") {
                    bootstrap_complete = true;
                    break;
                }
            }
        }
        
        if !bootstrap_complete {
            // Lire stderr pour les erreurs
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    log::error!("TOR stderr: {}", line);
                }
            }
            return Err(anyhow::anyhow!("TOR n'a pas réussi à démarrer"));
        }
        
        // Stocker le processus
        self.process = Some(child);
        
        // Attendre un peu que TOR soit complètement prêt
        std::thread::sleep(Duration::from_secs(2));
        
        // Mettre à jour la liste des nœuds de sortie
        self.refresh_exit_nodes()?;
        
        Ok(())
    }
    
    /// Arrête le daemon TOR
    pub async fn stop_tor(&mut self) -> Result<()> {
        if let Some(mut process) = self.process.take() {
            process.kill()?;
            process.wait()?;
        }
        Ok(())
    }
    
    /// Trouve le chemin vers l'exécutable TOR
    fn find_tor_executable(&self) -> Result<PathBuf> {
        // Chemins possibles pour TOR
        let possible_paths = [
            "/usr/local/bin/tor",
            "/usr/bin/tor",
            "/opt/homebrew/bin/tor", // macOS (Homebrew)
            "C:/Program Files/Tor/tor.exe", // Windows
            "C:/Tor Browser/Browser/TorBrowser/Tor/tor.exe", // Windows (Tor Browser)
        ];
        
        for path in possible_paths.iter() {
            let path_buf = PathBuf::from(path);
            if path_buf.exists() {
                return Ok(path_buf);
            }
        }
        
        // Essayer de trouver TOR dans le PATH
        if let Ok(path) = which::which("tor") {
            return Ok(path);
        }
        
        Err(anyhow::anyhow!(
            "TOR n'est pas installé ou introuvable. \
             Veuillez installer TOR ou spécifier le chemin dans les paramètres."
        ))
    }
    
    /// Rafraîchit la liste des nœuds de sortie TOR
    pub async fn refresh_exit_nodes(&mut self) -> Result<()> {
        use std::net::TcpStream;
        use std::io::{Write, BufRead};
        
        if self.process.is_none() {
            return Err(anyhow::anyhow!("TOR n'est pas démarré"));
        }
        
        // Se connecter au port de contrôle de TOR
        let control_addr = format!("127.0.0.1:{}", self.control_port);
        let mut stream = TcpStream::connect(&control_addr)
            .with_context(|| format!("Impossible de se connecter à TOR sur {}", control_addr))?;
        
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        
        // Authentification (pas de mot de passe par défaut)
        let auth_command = "AUTHENTICATE\r\n";
        stream.write_all(auth_command.as_bytes())?;
        
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        
        if !line.starts_with("250") {
            return Err(anyhow::anyhow!("Échec de l'authentification TOR: {}", line));
        }
        
        // Récupérer la liste des nœuds de sortie
        let get_nodes_command = "GETINFO ip-to-country/\r\n";
        stream.write_all(get_nodes_command.as_bytes())?;
        
        let mut node_info = String::new();
        reader.read_line(&mut node_info)?;
        
        if !node_info.starts_with("250") {
            return Err(anyhow::anyhow!("Échec de la récupération des nœuds: {}", node_info));
        }
        
        // Parser les informations des nœuds
        self.exit_nodes.clear();
        
        // Récupérer la liste complète des nœuds
        let get_all_nodes_command = "GETINFO ns/all\r\n";
        stream.write_all(get_all_nodes_command.as_bytes())?;
        
        let mut all_nodes = String::new();
        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line)?;
            if bytes_read == 0 {
                break;
            }
            if line.starts_with("250") && line.ends_with("\r\n") {
                continue;
            }
            if line == "250 OK\r\n" {
                break;
            }
            all_nodes.push_str(&line);
        }
        
        // Parser les nœuds
        for node_line in all_nodes.lines() {
            if node_line.trim().is_empty() {
                continue;
            }
            
            let parts: Vec<&str> = node_line.split_whitespace().collect();
            if parts.len() >= 4 {
                let nickname = parts[0].to_string();
                let fingerprint = parts[1].to_string();
                let ip = parts[2].to_string();
                let country_code = parts[3].to_string();
                
                // Obtenir le nom du pays à partir du code
                let country = self.country_code_to_name(&country_code);
                
                self.exit_nodes.push(TorExitNode {
                    nickname,
                    fingerprint,
                    ip,
                    country,
                    country_code,
                    is_running: true,
                    last_seen: None,
                });
            }
        }
        
        self.last_exit_nodes_update = Some(Instant::now());
        
        // Mettre à jour le cache par pays
        self.update_node_cache();
        
        Ok(())
    }
    
    /// Met à jour le cache des nœuds par pays
    fn update_node_cache(&mut self) {
        self.node_cache.clear();
        
        for node in &self.exit_nodes {
            self.node_cache
                .entry(node.country_code.clone())
                .or_insert_with(Vec::new)
                .push(node.clone());
        }
    }
    
    /// Convertit un code pays en nom de pays
    fn country_code_to_name(&self, code: &str) -> String {
        // Liste des codes pays -> noms (simplifiée)
        let countries: std::collections::HashMap<&str, &str> = [
            ("US", "États-Unis"),
            ("FR", "France"),
            ("DE", "Allemagne"),
            ("GB", "Royaume-Uni"),
            ("RU", "Russie"),
            ("CN", "Chine"),
            ("JP", "Japon"),
            ("CA", "Canada"),
            ("AU", "Australie"),
            ("BR", "Brésil"),
            ("IN", "Inde"),
        ]
        .iter()
        .cloned()
        .collect();
        
        countries.get(code).cloned().unwrap_or_else(|| code.to_string())
    }
    
    /// Sélectionne le meilleur nœud de sortie pour une destination donnée
    pub async fn select_exit_node(&self, target_country: Option<&str>) -> Option<TorExitNode> {
        // Si un pays cible est spécifié, essayer de trouver un nœud dans ce pays
        if let Some(country_code) = target_country {
            if let Some(nodes) = self.node_cache.get(country_code) {
                if !nodes.is_empty() {
                    return Some(nodes[0].clone()); // Retourner le premier nœud du pays
                }
            }
        }
        
        // Sinon, retourner un nœud aléatoire
        if !self.exit_nodes.is_empty() {
            let index = rand::random::<usize>() % self.exit_nodes.len();
            return Some(self.exit_nodes[index].clone());
        }
        
        None
    }
    
    /// Crée un client HTTP configuré pour utiliser TOR
    pub async fn create_tor_client(&self) -> Result<Client> {
        let proxy = Proxy::all(format!("socks5://127.0.0.1:{}", self.socks_port))?;
        
        let client = Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(30))
            .build()?;
        
        Ok(client)
    }
    
    /// Teste la connexion TOR
    pub async fn test_tor_connection(&self) -> Result<bool> {
        let client = self.create_tor_client()?;
        
        // Tester avec un site qui retourne l'IP
        let response = client
            .get("https://check.torproject.org/api/ip")
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let body = resp.text().await?;
                    let json: serde_json::Value = serde_json::from_str(&body)?;
                    
                    if let Some(ip) = json.get("ip").and_then(|v| v.as_str()) {
                        // Vérifier que l'IP n'est pas une IP locale
                        if ip.starts_with("192.168.") || ip.starts_with("10.") || ip == "127.0.0.1" {
                            return Ok(false);
                        }
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Err(_) => Ok(false),
        }
    }
    
    /// Récupère le statut actuel de TOR
    pub async fn get_status(&self) -> TorStatus {
        TorStatus {
            is_running: self.process.is_some(),
            version: None, // À implémenter
            socks_port: self.socks_port,
            control_port: self.control_port,
            bootstrap_status: None, // À implémenter
            circuits: 0, // À implémenter
            streams: 0, // À implémenter
        }
    }
    
    /// Récupère la liste des nœuds de sortie
    pub async fn get_exit_nodes(&self) -> Vec<TorExitNode> {
        self.exit_nodes.clone()
    }
    
    /// Récupère les nœuds de sortie pour un pays spécifique
    pub async fn get_exit_nodes_by_country(&self, country_code: &str) -> Vec<TorExitNode> {
        self.node_cache
            .get(country_code)
            .cloned()
            .unwrap_or_default()
    }
}

/// Client HTTP configuré pour utiliser TOR
#[derive(Clone)]
pub struct TorClient {
    pub client: Client,
    pub socks_port: u16,
}

impl TorClient {
    pub async fn new(socks_port: u16) -> Result<Self> {
        let proxy = Proxy::all(format!("socks5://127.0.0.1:{}", socks_port))?;
        
        let client = Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(30))
            .build()?;
        
        Ok(Self { client, socks_port })
    }
    
    pub async fn get(&self, url: &str) -> Result<reqwest::Response> {
        self.client.get(url).send().await
    }
    
    pub async fn post(&self, url: &str, body: serde_json::Value) -> Result<reqwest::Response> {
        self.client.post(url).json(&body).send().await
    }
    
    pub async fn with_exit_node(self, node: &TorExitNode) -> Result<Self> {
        // Pour l'instant, on ne peut pas changer de nœud de sortie dynamiquement
        // sans redémarrer TOR avec une configuration spécifique.
        // Cela nécessiterait une intégration plus poussée avec le contrôleur TOR.
        Ok(self)
    }
}

/// Commandes Tauri pour TOR


#[allow(dead_code)]
pub async fn get_tor_status(
    
        
        
            
        state: &crate::AppState) -> Result<TorStatus, String> {
    let tor_manager = state.tor_manager.lock().await;
    Ok(tor_manager.get_status())
}


#[allow(dead_code)]
pub async fn get_tor_exit_nodes(
    
        
        
            
        state: &crate::AppState) -> Result<Vec<TorExitNode>, String> {
    let tor_manager = state.tor_manager.lock().await;
    Ok(tor_manager.get_exit_nodes())
}


#[allow(dead_code)]
pub async fn test_tor_connection(
    
        
        
            
        state: &crate::AppState) -> Result<bool, String> {
    let tor_manager = state.tor_manager.lock().await;
    tor_manager.test_tor_connection().await.map_err(|e| e.to_string())
}


#[allow(dead_code)]
pub async fn refresh_tor_exit_nodes(
    
        
        
            
        state: &crate::AppState) -> Result<Vec<TorExitNode>, String> {
    let mut tor_manager = state.tor_manager.lock().await;
    tor_manager.refresh_exit_nodes()?;
    Ok(tor_manager.get_exit_nodes())
}
