# État actuel du projet - Phase 3

## Contexte

Nous sommes en train de corriger les erreurs de compilation pour permettre au backend de compiler sans dépendance à Tauri. Cela permettra de tester le backend de manière isolée.

## Problèmes identifiés

### Erreurs principales (425 au total)

1. **67 erreurs E0618**: `expected function, found &AppState` - Les fonctions attendent un `AppState` mais reçoivent une référence `&AppState`
2. **61 erreurs E0425**: `cannot find value 'state' in this scope` - Références à `state` non résolues
3. **21 erreurs E0308**: `mismatched types` - Incompatibilités de types
4. **16 erreurs E0433**: `cannot find type 'EntityType' in this scope` - Type EntityType non importé
5. **13 erreurs E0618**: `expected function, found &EntityManager` - Problèmes de signatures
6. **10 erreurs E0618**: `expected function, found &GererManager`
7. **10 erreurs E0616**: `field 'client' of struct 'TorClient' is private` - Champs privés
8. **7 erreurs E0728**: `await is only allowed inside async functions` - await en dehors de contexte async
9. **7 erreurs E0618**: `expected function, found &WatchManager`
10. **5 erreurs E0618**: `expected function, found &WatchScheduler`

## Corrections nécessaires

### 1. Rendre les champs publics
- Dans `TorClient`: rendre `client` public
- Dans `Database`: rendre `conn` public

### 2. Corriger les signatures de fonctions
- Toutes les fonctions qui prennent `state: &crate::AppState` doivent être marquées `async`
- Les fonctions qui appellent `.await` doivent être `async`

### 3. Importer EntityType
- Ajouter `use crate::backend::entities::EntityType;` dans les fichiers qui l'utilisent

### 4. Corriger les références `state`
- Dans les fonctions, `state` doit être un paramètre, pas une variable globale
- Remplacer `state.X.lock().await` par `(&state.X).lock().await`

### 5. Corriger les appels avec self
- Remplacer `self.X.lock().await` par `(&self.X).lock().await`

## Stratégie

Plutôt que de corriger chaque fichier individuellement, nous allons:
1. Créer un script Python qui applique toutes les corrections systématiquement
2. Corriger manuellement les cas particuliers
3. Tester la compilation après chaque étape

## Fichiers à corriger en priorité

1. **db.rs** - Rendre `conn` public
2. **tor.rs** - Rendre `client` public, corriger `create_tor_client`
3. **entities.rs** - Importer EntityType
4. **main.rs** - Déjà corrigé

## Commandes utiles

```bash
# Vérifier le nombre d'erreurs
cargo check --no-default-features 2>&1 | grep "error\[" | wc -l

# Voir les erreurs par type
cargo check --no-default-features 2>&1 | grep "error\[" | sort | uniq -c | sort -rn | head -20

# Voir les erreurs d'un type spécifique
cargo check --no-default-features 2>&1 | grep "E0618"
```
