# 📖 Guide d'Installation et de Configuration - OSINT App

> **Version** : 0.1.0  
> **Dernière mise à jour** : 2024  
> **Auteur** : Équipe OSINT

---

## 📌 Table des Matières

1. [🔧 Prérequis](#-prérequis)
2. [🚀 Installation](#-installation)
3. [⚙️ Configuration](#-configuration)
   - [TOR](#tor)
   - [MLA (Chiffrement ANSSI)](#mla-chiffrement-anssi)
   - [Clés et Secrets](#clés-et-secrets)
   - [Base de Données](#base-de-données)
   - [API Externes](#api-externes)
4. [🏃 Lancement](#-lancement)
5. [🧪 Tests](#-tests)
6. [📊 Utilisation de Base](#-utilisation-de-base)
7. [🐛 Résolution des Problèmes](#-résolution-des-problèmes)
8. [📚 Ressources Utiles](#-ressources-utiles)

---


## 🔧 1. Prérequis

### ⚠️ Exigences Système

| **Éléments** | **Version Minimale** | **Vérification** |
|--------------|----------------------|-----------------|
| **Système d'exploitation** | macOS 10.13 (High Sierra) ou supérieur | `sw_vers` |
| **Architecture** | x86_64 ou arm64 (Apple Silicon) | `uname -m` |
| **Espace disque disponible** | 500 Mo | `df -h` |
| **Mémoire RAM** | 4 Go (8 Go recommandé) | `sysctl hw.memsize` |

### 🛠️ Logiciels Requises

| **Logiciel** | **Version** | **Installation** | **Vérification** |
|--------------|-------------|-----------------|-----------------|
| **Rust** | 1.70+ | [https://rustup.rs](https://rustup.rs) | `rustc --version` |
| **Node.js** | 18+ | [https://nodejs.org](https://nodejs.org) ou `nvm` | `node --version` |
| **npm** | 8+ | Inclus avec Node.js | `npm --version` |
| **TOR** | 0.4.7+ | `brew install tor` | `tor --version` |
| **SQLite** | 3.35+ | Inclus avec macOS | `sqlite3 --version` |
| **Git** | 2.30+ | `brew install git` | `git --version` |
| **CMake** | 3.15+ | `brew install cmake` | `cmake --version` |

---


## 🚀 2. Installation

### Étape 1 : Cloner le Dépôt

```bash
# Cloner le dépôt principal
git clone https://github.com/reSqur-dygcus-wumvu8/V3.git
cd V3

# Passer sur la branche principale (si nécessaire)
git checkout main
```

---

### Étape 2 : Installer les Dépendances Globales

#### Installer Tauri CLI

```bash
# Installer Tauri CLI globalement
npm install --global @tauri-apps/cli

# Vérifier l'installation
tauri --version
```

#### Installer les Outils Rust

```bash
# Installer les outils de build Rust
cargo install tauri-cli cargo-bindgen

# Vérifier
cargo --version
```

---

### Étape 3 : Configurer l'Application Principale

```bash
# Se placer dans le dossier de l'application principale
cd app/main

# Installer les dépendances npm (frontend)
npm install

# Builder le frontend (Svelte + TypeScript)
npm run build

# Builder le backend (Rust)
cargo build

# Vérifier le build
ls -la target/debug/  # Doit contenir l'exécutable
```

---

### Étape 4 : Configurer l'Application Secrets

```bash
# Se placer dans le dossier de l'application Secrets
cd ../secrets

# Installer les dépendances npm
npm install

# Builder le frontend
npm run build

# Builder le backend
cargo build
```

---

### Étape 5 : Installer MLA (Optionnel pour le développement)

> **⚠️ Important** : En mode **développement**, l'application utilise des **stubs** pour MLA (chiffrement simulé). Pour une utilisation en **production**, MLA doit être installé.

```bash
# Cloner le dépôt MLA (ANSSI)
git clone https://github.com/ANSSI-FR/MLA.git
cd MLA

# Créer un dossier de build
mkdir build && cd build

# Configurer avec CMake
cmake ..

# Compiler
make

# Installer (optionnel, nécessite sudo)
sudo make install

# Retourner au projet
cd ../../V3
```

---


## ⚙️ 3. Configuration

---

### 🌐 3.1 Configuration de TOR

L'application peut **lancer TOR automatiquement** ou se connecter à un **daemon TOR existant**.

#### Option 1 : Lancement Automatique (Recommandé)

- **Aucune configuration requise** : L'application lancera TOR avec les paramètres par défaut.
- **Ports utilisés** :
  - **SOCKS5** : `9050` (pour les requêtes HTTP via TOR)
  - **Control** : `9051` (pour la gestion du daemon)
  - **Data Directory** : `/tmp/tor_data` (par défaut)

#### Option 2 : Utiliser un Daemon TOR Existant

1. **Lancer TOR manuellement** :
   ```bash
   # Lancer TOR avec les ports par défaut
   tor --SOCKSPort 9050 --ControlPort 9051 --DataDirectory /tmp/tor_data
   ```

2. **Dans l'application** :
   - Aller dans **Paramètres > TOR**
   - Désactiver **"Démarrer TOR automatiquement"**
   - Vérifier que les ports **9050** (SOCKS5) et **9051** (Control) sont corrects

#### Sélection des Nœuds de Sortie

L'application **sélectionne automatiquement** un nœud de sortie dans le **pays de destination** :

| **Destination** | **Pays du Nœud** | **Exemple** |
|----------------|------------------|-------------|
| Google Actualités | États-Unis (`US`) | `google.com` |
| Qwant Actualités | France (`FR`) | `qwant.com` |
| Yandex Actualités | Russie (`RU`) | `yandex.ru` |
| Twitter/X | États-Unis (`US`) | `twitter.com` |
| Telegram | Divers (selon le serveur) | `telegram.org` |

- **Fallback** : Si aucun nœud adapté n'est disponible, un nœud par défaut est utilisé.
- **Choix manuel** : L'utilisateur peut sélectionner un nœud spécifique dans **Paramètres > TOR > Nœuds de sortie**.

#### Vérifier la Connexion TOR

1. Dans l'application, aller dans **Paramètres > TOR**
2. Cliquer sur **"Tester la connexion"**
3. **Résultat attendu** :
   - ✅ **"Connecté"** + IP TOR (ex: `185.220.101.XXX`)
   - ❌ **"Déconnecté"** si TOR n'est pas lancé ou mal configuré

---


### 🔐 3.2 Configuration de MLA (Chiffrement ANSSI)

**MLA (Mécanisme de Chiffrement de l'ANSSI)** est utilisé pour le **chiffrement de bout en bout** de la base de données.

#### Installation de MLA

1. **Cloner le dépôt** :
   ```bash
   git clone https://github.com/ANSSI-FR/MLA.git
   cd MLA
   ```

2. **Compiler MLA** :
   ```bash
   mkdir build && cd build
   cmake ..
   make
   ```

3. **Installer la bibliothèque** (optionnel) :
   ```bash
   sudo make install
   ```

4. **Copier les fichiers nécessaires** :
   ```bash
   # Copier le header dans le projet
   cp include/mla.h ../../V3/app/main/src/backend/mla/
   
   # Copier la bibliothèque (macOS)
   cp libmla.dylib ../../V3/app/main/src/backend/mla/lib/
   ```

#### Configuration dans l'Application

1. **Définir le chemin du header MLA** :
   ```bash
   # Variable d'environnement (pour bindgen)
   export MLA_HEADER_PATH=/chemin/vers/MLA/include/mla.h
   ```

2. **Mettre à jour `app/main/build.rs`** :
   ```rust
   let mla_header = PathBuf::from(env::var("MLA_HEADER_PATH").unwrap());
   ```

3. **Rebuilder le projet** :
   ```bash
   cd app/main
   cargo clean
   cargo build
   ```

> **⚠️ Note** : 
> - En **mode développement**, l'application utilise des **stubs** (simulation de chiffrement).
> - En **mode production**, MLA doit être correctement installé pour un chiffrement **ANSSI-compliant**.

---


### 🗝️ 3.3 Configuration des Clés et Secrets

L'**application Secrets** gère toutes les **clés de chiffrement** et **tokens API** de manière sécurisée via le **Keychain macOS**.

#### Première Utilisation

1. **Lancer l'application Secrets** :
   ```bash
   cd app/secrets
   cargo tauri run
   ```

2. **Générer une Clé Maître** :
   - Cliquer sur **"Générer une clé maître"**
   - La clé est **automatiquement sauvegardée** dans le Keychain macOS
   - **⚠️ Ne jamais partager cette clé** (elle est utilisée pour déchiffrer toute la base de données)

3. **Ajouter les Tokens API** :
   - Aller dans **Tokens API**
   - Saisir votre **clé API Mistral** (obtenue via [Mistral AI](https://mistral.ai/))
   - **Optionnel** : Ajouter d'autres tokens (Google, Qwant, etc.)

#### Communication entre les Applications

- **Mécanisme** : **Unix Domain Sockets** (communication locale sécurisée)
- **Chemin du socket** : `~/Library/Application Support/osint_app/secrets.sock`
- **Clé secrète partagée** : Générée automatiquement et stockée dans le Keychain
- **Chiffrement** : Tous les messages IPC sont **chiffrés (AES-256-GCM)** et **signés (HMAC-SHA256)**

#### Gestion des Clés

| **Clé** | **Stockage** | **Utilisation** |
|---------|-------------|----------------|
| **Clé maître** | Keychain macOS | Chiffrement de la clé de session |
| **Clé de session** | Mémoire (app principale) | Chiffrement des données |
| **Clé SQLCipher** | Fichier chiffré (MLA) | Chiffrement de la base SQLite |
| **Token Mistral API** | Keychain macOS | Appels à l'API Mistral |

---


### 🗃️ 3.4 Configuration de la Base de Données

- **Type** : **SQLite** (avec extension **SQLCipher** pour le chiffrement)
- **Emplacement par défaut** : `~/Library/Application Support/osint_app/osint_app.db`
- **Chiffrement** : **Double couche** (SQLCipher + MLA)

#### Schéma de la Base

La base contient les tables suivantes :

| **Table** | **Description** |
|-----------|-----------------|
| `entity_types` | Types d'entités (individu, organisation, etc.) |
| `individuals` | Individus (réels, fictifs, numériques) |
| `organizations` | Organisations |
| `events` | Événements |
| `locations` | Lieux (réels, fictifs, numériques) |
| `objects` | Objets |
| `sources` | Sources d'information |
| `source_ratings` | Cotations des sources (A-F) |
| `info_ratings` | Cotations des informations (1-6) |
| `watches` | Configuration des veilles |
| `watch_results` | Résultats des veilles |
| `capitalized_infos` | Informations capitalisées |
| `imported_files` | Fichiers importés |
| `entity_relations` | Relations entre entités |
| `audit_log` | Historique des actions (audit) |
| `social_accounts` | Comptes réseaux sociaux |
| `duplicate_groups` | Groupes de doublons |
| `acl_cache` | Cache des données ACLED |

#### Sauvegarde de la Base

1. **Sauvegarde manuelle** :
   - Copier le fichier `osint_app.db` vers un emplacement sûr
   - **⚠️ Le fichier est chiffré** : ne peut être ouvert sans la clé MLA

2. **Sauvegarde automatique** (à implémenter) :
   - Configurable dans **Paramètres > Base de données**
   - Fréquence : Quotidienne, Hebdomadaire, Mensuelle

---


### 🔗 3.5 Configuration des API Externes

#### Mistral API (Requis)

1. **Obtenir une clé API** :
   - S'inscrire sur [Mistral AI](https://mistral.ai/)
   - Aller dans **Compte > Clés API**
   - Générer une nouvelle clé

2. **Configurer dans l'application Secrets** :
   - Lancer l'app **Secrets**
   - Aller dans **Tokens API**
   - Saisir la clé sous le nom **`mistral`**

3. **Paramètres par défaut** :
   - **Endpoint** : `https://api.mistral.ai/v1`
   - **Modèle** : `mistral-large` (recommandé pour le NER)

#### ACLED API (Optionnel)

1. **Obtenir un compte** : [https://acleddata.com](https://acleddata.com)
2. **Configurer l'endpoint** :
   - Dans **Paramètres > API Externes**
   - **Endpoint ACLED** : `https://api.acleddata.com`

---


## 🏃 4. Lancement

---

### 4.1 Lancer l'Application Secrets (Obligatoire)

> **⚠️ Important** : L'application **Secrets** doit être lancée **avant** l'application principale pour que la communication IPC fonctionne.

```bash
cd app/secrets
cargo tauri run
```

- **Fenêtre** : Une interface graphique s'ouvre pour gérer les clés et tokens.
- **Statut** : Vérifier que le **serveur IPC est actif** (indiqué dans l'UI).

---


### 4.2 Lancer l'Application Principale

```bash
cd app/main
cargo tauri run
```

- **Fenêtre** : L'application principale s'ouvre avec la page d'accueil.
- **Statut** : Vérifier que **TOR** et la **base de données** sont connectés.

---


### 4.3 Lancer en Mode Production

Pour générer un **binaire macOS** (`.app`) :

```bash
# Pour l'application principale
cd app/main
cargo tauri build

# Pour l'application Secrets
cd ../secrets
cargo tauri build
```

- **Sortie** : Les fichiers `.app` sont générés dans :
  - `app/main/target/release/bundle/macos/OSINT App.app`
  - `app/secrets/target/release/bundle/macos/OSINT Secrets.app`

---


## 🧪 5. Tests

---

### 5.1 Tests du Backend (Rust)

```bash
cd app/main
cargo test
```

**Tests inclus** :
- ✅ Connexion à la base de données
- ✅ Chiffrement/déchiffrement MLA (stubs en dev)
- ✅ Sélection des nœuds TOR
- ✅ Déduplication des fichiers (hash SHA-256)

---


### 5.2 Tests du Frontend (Svelte)

```bash
cd app/main
npm run test
```

> **⚠️ À implémenter** : Tests E2E avec Playwright ou Vitest.

---


### 5.3 Tests Manuels

#### Test 1 : Connexion TOR

1. **Lancer TOR** :
   ```bash
   tor
   ```
2. **Dans l'application** :
   - Aller dans **Paramètres > TOR**
   - Cliquer sur **"Tester la connexion"**
   - **Résultat attendu** : ✅ **"Connecté"** + IP TOR

#### Test 2 : Import de Fichiers

1. **Glisser-déposer** un fichier PDF/TXT dans **Acquérir > Importer des fichiers**
2. **Sélectionner une cotation** (ex: B - Généralement fiable)
3. **Cliquer sur "Importer les fichiers"**
   - **Résultat attendu** : Le fichier est importé et apparaît dans la liste

#### Test 3 : Détection de Doublons

1. **Importer le même fichier deux fois**
   - **Résultat attendu** : Le 2ème import est **ignoré** (message "Fichier déjà importé")

#### Test 4 : Configuration d'une Veille

1. **Aller dans Acquérir > Veille Internet**
2. **Créer une veille** :
   - Nom : `Test Veille`
   - Requête : `Cybersécurité 2024`
   - Sources : Google Actualités, Qwant
   - Fréquence : 1 heure
3. **Cliquer sur "Ajouter la veille"**
   - **Résultat attendu** : La veille apparaît dans la liste

#### Test 5 : Mode Découverte

1. **Aller dans Acquérir > Mode Découverte**
2. **Saisir une thématique** : `Conflits en Afrique de l'Ouest`
3. **Cliquer sur "Explorer"**
   - **Résultat attendu** : Une synthèse + sources + mots-clés (via Mistral API)

#### Test 6 : Gestion des Clés (App Secrets)

1. **Lancer l'app Secrets**
2. **Générer une clé maître**
   - **Résultat attendu** : Clé générée et sauvegardée dans le Keychain
3. **Ajouter un token API** (ex: Mistral)
   - **Résultat attendu** : Token sauvegardé dans le Keychain

---


## 📊 6. Utilisation de Base

---

### 6.1 Module ACQUÉRIR

#### Importer des Fichiers

1. **Glisser-déposer** des fichiers dans la zone dédiée
2. **Sélectionner une source** (ex: "Import local", "Téléchargement")
3. **Choisir une cotation** (A-F pour la fiabilité de la source)
4. **Cliquer sur "Importer"**

**Fonctionnalités** :
- ✅ Déduplication automatique (hash SHA-256)
- ✅ Stockage des métadonnées (nom, date, source, cotation)
- ✅ Lien optionnel à une veille existante

---

#### Configurer une Veille

1. **Aller dans Acquérir > Veille Internet**
2. **Cliquer sur "Nouvelle veille"**
3. **Remplir les champs** :
   - **Nom** : Identifiant de la veille (ex: "Veille Ukraine")
   - **Requête** : Mots-clés de recherche (ex: "Guerre Ukraine 2024")
   - **Langue** : Langue de la requête (français par défaut)
   - **Sources** : Sélectionner les moteurs (Google, Qwant, Yandex, Twitter, etc.)
   - **Fréquence** : Intervalle de mise à jour (5 min à 1 jour)
4. **Activer la veille**

**Fonctionnalités** :
- ✅ Exécution automatique selon la fréquence
- ✅ Requêtes via **TOR** (nœud de sortie adapté)
- ✅ Traduction automatique en français (via Mistral API)
- ✅ Détection d'entités (NER via Mistral API)

---

#### Mode Découverte

1. **Saisir une thématique** (ex: "Cybermenaces 2024")
2. **Choisir les paramètres** :
   - Langue de la requête
   - Température (0.1 à 0.9 pour la créativité)
   - Nombre de tokens (500 à 4000)
3. **Cliquer sur "Explorer"**

**Résultats** :
- 📄 **Synthèse structurée** (500-1000 mots)
- 🔗 **Sources proposées** (URL + nom)
- 🏷️ **Mots-clés** pour les veilles

---


### 6.2 Module CAPITALISER

> **⚠️ À implémenter dans les prochaines itérations**

Fonctionnalités prévues :
- Regroupement des informations par événements
- Proposition automatique de cotation (source × information)
- Reconnaissance d'entités (NER) et liaison à la base de connaissance
- Création de fiches d'entités (individus, organisations, événements, etc.)
- Recherche internet automatique pour compléter les fiches

---


### 6.3 Module EXPLOITER

> **⚠️ À implémenter dans les prochaines itérations**

Fonctionnalités prévues :
- Affichage des fiches d'entités
- Graphes relationnels interactifs (Cytoscape.js)
- Frise chronologique (Vis.js)
- Cartes avec fond OpenStreetMap/Esri/Sentinel + cache hors ligne
- Affichage des événements ACLED

---


### 6.4 Module GÉRER

> **⚠️ À implémenter dans les prochaines itérations**

Fonctionnalités prévues :
- Détection automatique des doublons (appariement flou + sémantique)
- File de revue des doublons avec validation explicite
- Fusion assistée des entités
- Gestion des paramètres (TOR, cartes, API)

---


## 🐛 7. Résolution des Problèmes

---

### ❌ Problèmes Courants et Solutions

| **Problème** | **Cause Possible** | **Solution** |
|--------------|-------------------|--------------|
| **TOR ne démarre pas** | TOR non installé | `brew install tor` |
| | Ports occupés | Vérifier avec `lsof -i :9050` |
| | Permission refusée | Lancer avec `sudo` ou vérifier les permissions |
| **Erreur de connexion TOR** | TOR non lancé | Lancer TOR avec `tor` |
| | Mauvais port | Vérifier les ports dans **Paramètres > TOR** |
| | Nœuds bloqués | Changer de nœud de sortie manuellement |
| **MLA non trouvé** | MLA non installé | Suivre [Configuration de MLA](#mla-chiffrement-anssi) |
| | Header introuvable | Définir `MLA_HEADER_PATH` |
| **Base de données corrompue** | Fermeture brutale | Supprimer `osint_app.db` et relancer |
| | Clé incorrecte | Régénérer la clé maître dans l'app Secrets |
| **Communication IPC échouée** | App Secrets non lancée | Lancer l'app Secrets **avant** l'app principale |
| | Socket introuvable | Vérifier le chemin dans **Paramètres > IPC** |
| **Erreur de permissions** | Accès refusé au Keychain | Autoriser dans **Préférences Système > Sécurité** |
| | Accès refusé au fichier | Vérifier les permissions avec `chmod` |
| **Mistral API échoue** | Clé API invalide | Vérifier la clé dans l'app Secrets |
| | Quota dépassé | Vérifier votre quota sur [Mistral AI](https://mistral.ai/) |
| **Scraping échoue** | Site bloqué | Utiliser un autre nœud TOR |
| | Structure HTML changée | Mettre à jour le scraper |

---


### 🔍 Dépannage Avancé

#### Vérifier les Logs

Les logs sont stockés dans :
- **Application principale** : `~/Library/Logs/osint_app/main.log`
- **Application Secrets** : `~/Library/Logs/osint_app/secrets.log`
- **TOR** : `/tmp/tor_data/log` (si lancé automatiquement)

**Afficher les logs** :
```bash
# Logs de l'application principale
tail -f ~/Library/Logs/osint_app/main.log

# Logs de TOR
tail -f /tmp/tor_data/log
```

---


#### Tester la Connexion TOR en Ligne de Commande

```bash
# Vérifier si TOR est en cours d'exécution
ps aux | grep tor

# Tester la connexion via curl (avec proxy SOCKS5)
curl --socks5 127.0.0.1:9050 https://check.torproject.org/api/ip
# Résultat attendu : {"ip":"185.220.101.XXX"}
```

---


#### Tester la Base de Données

```bash
# Se connecter à la base (nécessite SQLCipher)
sqlite3 ~/Library/Application\ Support/osint_app/osint_app.db

# Vérifier les tables
.tables

# Exécuter une requête (si non chiffrée)
SELECT * FROM entity_types;
```

> **⚠️ Note** : Si la base est chiffrée avec SQLCipher, il faut fournir la clé :
> ```bash
> sqlite3 -key "votre-clé-sqlcipher" osint_app.db
> ```

---


#### Réinitialiser l'Application

```bash
# Supprimer la base de données
rm -rf ~/Library/Application\ Support/osint_app/osint_app.db

# Supprimer les paramètres
rm -rf ~/Library/Application\ Support/osint_app/settings.json

# Supprimer le cache TOR
rm -rf /tmp/tor_data

# Relancer l'application
cd app/main
cargo tauri run
```

---


## 📚 8. Ressources Utiles

---

### 🔗 Liens Externes

| **Ressource** | **Lien** | **Description** |
|---------------|----------|-----------------|
| **Site Tauri** | [https://tauri.app](https://tauri.app) | Documentation officielle Tauri |
| **Dépôt MLA** | [https://github.com/ANSSI-FR/MLA](https://github.com/ANSSI-FR/MLA) | Bibliothèque de chiffrement ANSSI |
| **Documentation TOR** | [https://www.torproject.org/docs](https://www.torproject.org/docs) | Documentation officielle TOR |
| **API Mistral** | [https://docs.mistral.ai](https://docs.mistral.ai) | Documentation de l'API Mistral |
| **ACLED Data** | [https://acleddata.com](https://acleddata.com) | Données sur les conflits armés |
| **SQLCipher** | [https://www.zetetic.net/sqlcipher/](https://www.zetetic.net/sqlcipher/) | Extension SQLite pour le chiffrement |
| **Leaflet** | [https://leafletjs.com](https://leafletjs.com) | Bibliothèque de cartes interactive |
| **Cytoscape.js** | [https://js.cytoscape.org](https://js.cytoscape.org) | Bibliothèque de graphes interactifs |

---


### 📖 Documentation Interne

| **Document** | **Chemin** | **Description** |
|--------------|------------|-----------------|
| **Architecture** | `docs/architecture.md` | Architecture complète de l'application |
| **Setup** | `docs/setup.md` | **Ce document** |
| **API Interne** | `docs/api.md` | Documentation des commandes Tauri |
| **Modèle de Données** | `app/main/src/backend/db.rs` | Schéma SQLite complet |

---


### 💬 Support

Pour toute question ou problème, contacter :
- **Équipe de développement** : [votre-email@organisation.com](mailto:votre-email@organisation.com)
- **Dépôt GitHub** : [https://github.com/reSqur-dygcus-wumvu8/V3](https://github.com/reSqur-dygcus-wumvu8/V3)

---


## 📜 Annexes

---

### A.1 Cotations OTAN

L'application utilise le **système de cotation OTAN** pour évaluer la **fiabilité des sources** et la **crédibilité des informations**.

#### Fiabilité des Sources (A-F)

| **Code** | **Libellé** | **Description** |
|----------|-------------|-----------------|
| A | Totalement fiable | Source toujours exacte, sans erreur connue |
| B | Généralement fiable | Source généralement exacte, erreurs mineures possibles |
| C | Assez fiable | Source généralement fiable, mais peut contenir des erreurs |
| D | Peu fiable | Source peu fiable, nécessite vérification |
| E | Fiabilité non évaluable | Impossible d'évaluer la fiabilité |
| F | Non évaluable | Source non évaluable |

#### Crédibilité de l'Information (1-6)

| **Code** | **Libellé** | **Description** |
|----------|-------------|-----------------|
| 1 | Confirmée | Information confirmée par plusieurs sources fiables |
| 2 | Probablement vraie | Information probablement vraie, mais non confirmée |
| 3 | Possiblement vraie | Information possiblement vraie, nécessite vérification |
| 4 | Douteuse | Information douteuse, peu probable |
| 5 | Improbable | Information probablement fausse |
| 6 | Non évaluable | Impossible d'évaluer la crédibilité |

---


### A.2 Types d'Entités

L'application gère les types d'entités suivants :

| **Type** | **Description** | **Exemples** |
|----------|-----------------|--------------|
| **Individu** | Personnes physiques ou entités | Volodymyr Zelensky, Elon Musk |
| **Organisation** | Groupes, entreprises, gouvernements | OTAN, Google, FBI |
| **Événement** | Incidents, conférences, attaques | Guerre en Ukraine, Conférence sur la cybersécurité |
| **Lieu** | Emplacements physiques ou numériques | Paris, 192.168.1.1 |
| **Objet** | Artefacts, documents, appareils | Rapport PDF, iPhone 15 |
| **Source** | Origine des informations | BBC, Twitter, Le Monde |

---


### A.3 Raccourcis Clavier

| **Raccourci** | **Action** | **Disponible dans** |
|--------------|------------|---------------------|
| `Ctrl + S` | Sauvegarder | Toutes les pages |
| `Ctrl + Z` | Annuler | Formulaires |
| `Ctrl + F` | Rechercher | Pages avec du contenu textuel |
| `Ctrl + N` | Nouvelle veille | Page Veille Internet |
| `Ctrl + I` | Importer un fichier | Page Import de fichiers |
| `Escape` | Fermer la modale | Toutes les modales |

---


### A.4 Variables d'Environnement

| **Variable** | **Description** | **Valeur par Défaut** |
|--------------|-----------------|----------------------|
| `MLA_HEADER_PATH` | Chemin vers `mla.h` | `/usr/local/include/mla.h` |
| `TOR_SOCKS_PORT` | Port SOCKS5 pour TOR | `9050` |
| `TOR_CONTROL_PORT` | Port de contrôle pour TOR | `9051` |
| `SQLCIPHER_KEY` | Clé pour SQLCipher | Générée automatiquement |
| `MISTRAL_API_KEY` | Clé API Mistral | À configurer dans l'app Secrets |
| `APP_CONFIG_DIR` | Dossier de configuration | `~/Library/Application Support/osint_app` |

---


## 🎯 Conclusion

Félicitations ! Vous avez maintenant une **application OSINT complète** opérationnelle, avec :

✅ **Chiffrement ANSSI** (MLA + SQLCipher)  
✅ **Connexion sécurisée via TOR** (nœuds par pays + fallback)  
✅ **Gestion des secrets** (Keychain macOS)  
✅ **Import de fichiers** (déduplication, cotation OTAN)  
✅ **Veille Internet** (Google, Qwant, Yandex, réseaux sociaux)  
✅ **Mode Découverte** (Mistral API)  
✅ **UI moderne** (Svelte + TailwindCSS)  

**Prochaines étapes** :
- [ ] Implémenter le **Module CAPITALISER** (base de connaissance, NER, gestion des doublons)
- [ ] Implémenter le **Module EXPLOITER** (graphes, frise chronologique, cartes)
- [ ] Implémenter le **Module GÉRER** (paramètres, doublons)

---

*Documentation générée automatiquement - Dernière mise à jour : 2024*
