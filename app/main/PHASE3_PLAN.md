# Plan Phase 3 - Correction des erreurs de compilation

## Objectif
Permettre au backend de compiler sans dépendance à Tauri.

## Stratégie
Plutôt que de corriger tous les fichiers à la fois, nous allons:
1. Créer un main.rs minimal qui compile
2. Corriger un module à la fois
3. Valider après chaque correction

## Étapes

### Étape 0: Préparation (DONE)
- [x] Créer une branche dédiée: `vibe/phase3-final`
- [x] Rendre les champs publics dans db.rs et tor.rs

### Étape 1: main.rs minimal
Créer un main.rs qui:
- N'utilise pas Tauri
- Initialise tous les composants de base
- Compile sans erreur

### Étape 2: Corriger les modules un par un
Pour chaque module backend:
1. Remplacer `#[tauri::command]` par `#[allow(dead_code)]`
2. Remplacer `tauri::State<AppState>` par `&crate::AppState`
3. Ajouter `use crate::AppState;`
4. Remplacer `.lock().unwrap()` par `.lock().await`
5. Rendre les fonctions `async`
6. Corriger les références `settings.shared.*` → `settings.*`
7. Importer EntityType depuis `osint_shared::models`
8. Corriger les appels avec `?` pour ajouter `.map_err(|e| e.to_string())`

### Étape 3: Corriger les références
- Remplacer `state.X.lock().await` par `(&state.X).lock().await`
- Remplacer `self.X.lock().await` par `(&self.X).lock().await`

## Fichiers par priorité

### Priorité 1 (Critiques)
- [ ] main.rs
- [ ] db.rs
- [ ] tor.rs
- [ ] entities.rs

### Priorité 2 (Importants)
- [ ] settings.rs
- [ ] api.rs
- [ ] ner.rs
- [ ] capitalize.rs
- [ ] gerer.rs

### Priorité 3 (Secondaires)
- [ ] watch/mod.rs
- [ ] watch/scheduler.rs
- [ ] watch/processor.rs
- [ ] watch/scrapers.rs
- [ ] importer.rs
- [ ] dedup.rs
- [ ] cotations.rs
- [ ] exploit.rs
- [ ] social.rs
- [ ] discovery.rs
- [ ] mla.rs

## Commandes utiles

```bash
# Vérifier le nombre d'erreurs
cargo check --no-default-features 2>&1 | grep "error\[" | wc -l

# Voir les erreurs par type
cargo check --no-default-features 2>&1 | grep "error\[" | sort | uniq -c | sort -rn | head -20

# Voir les erreurs dans un fichier spécifique
cargo check --no-default-features 2>&1 | grep "backend/db.rs"

# Tester la compilation d'un fichier spécifique
rustc --crate-type lib src/backend/db.rs --no-default-features -L target/debug/deps 2>&1 | head -20
```

## Checklist des corrections

- [ ] Tous les `#[tauri::command]` remplacés
- [ ] Tous les `tauri::State<...>` remplacés
- [ ] Tous les `.lock().unwrap()` remplacés par `.lock().await`
- [ ] Tous les `settings.shared.*` remplacés
- [ ] Tous les imports EntityType ajoutés
- [ ] Tous les `?` à la fin de ligne corrigés avec `.map_err(|e| e.to_string())`
- [ ] Toutes les fonctions rendues `async`
- [ ] Toutes les références `state.X` et `self.X` corrigées

## Temps estimé
- 2-3 heures pour corriger tous les fichiers
- 1 heure pour tester et valider
