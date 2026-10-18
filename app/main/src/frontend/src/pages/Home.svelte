<script lang="ts">
    import { Link } from 'svelte-routing';
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/tauri';
    
    let stats = {
        totalFiles: 0,
        totalWatches: 0,
        totalEntities: 0,
        lastUpdate: '',
    };
    
    let recentActivity: Array<{type: string, description: string, timestamp: string}> = [];
    let torStatus = 'Inconnu';
    let dbStatus = 'Inconnu';
    
    onMount(async () => {
        // Charger les statistiques
        try {
            // Ces commandes seront implémentées dans le backend
            // stats.totalFiles = await invoke('get_total_files');
            // stats.totalWatches = await invoke('get_total_watches');
            // stats.totalEntities = await invoke('get_total_entities');
            
            // Pour l'instant, on utilise des valeurs par défaut
            stats.totalFiles = 42;
            stats.totalWatches = 5;
            stats.totalEntities = 128;
            stats.lastUpdate = new Date().toLocaleString('fr-FR');
            
            // Vérifier le statut TOR
            torStatus = await invoke('get_tor_status')
                .then(() => 'Connecté')
                .catch(() => 'Déconnecté');
            
            // Vérifier le statut de la base de données
            dbStatus = await invoke('test_db_connection')
                .then((result: boolean) => result ? 'Chiffrée' : 'Erreur')
                .catch(() => 'Erreur');
            
            // Charger l'activité récente (mock)
            recentActivity = [
                { type: 'import', description: 'Fichier rapport_2024.pdf importé', timestamp: new Date().toLocaleString('fr-FR') },
                { type: 'watch', description: 'Nouveaux résultats pour la veille "Guerre Ukraine"', timestamp: new Date().toLocaleString('fr-FR') },
                { type: 'capitalize', description: '12 informations capitalisées', timestamp: new Date().toLocaleString('fr-FR') },
            ];
        } catch (error) {
            console.error('Erreur lors du chargement de la page d\'accueil:', error);
        }
    });
</script>

<div class="space-y-6">
    <div>
        <h1 class="text-3xl font-bold">Tableau de bord</h1>
        <p class="text-gray-400">Bienvenue dans l'application OSINT</p>
    </div>
    
    <!-- Cartes de statistiques -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <div class="flex items-center justify-between">
                <div>
                    <p class="text-sm text-gray-400">Fichiers importés</p>
                    <p class="text-3xl font-bold">{stats.totalFiles}</p>
                </div>
                <div class="bg-blue-600 p-3 rounded-full">
                    <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                        <polyline points="7 10 12 15 17 10"/>
                        <line x1="12" y1="15" x2="12" y2="3"/>
                    </svg>
                </div>
            </div>
        </div>
        
        <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <div class="flex items-center justify-between">
                <div>
                    <p class="text-sm text-gray-400">Veilles actives</p>
                    <p class="text-3xl font-bold">{stats.totalWatches}</p>
                </div>
                <div class="bg-green-600 p-3 rounded-full">
                    <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10"/>
                        <polyline points="12 6 12 12 12 18"/>
                        <line x1="6" y1="12" x2="18" y2="12"/>
                    </svg>
                </div>
            </div>
        </div>
        
        <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <div class="flex items-center justify-between">
                <div>
                    <p class="text-sm text-gray-400">Entités en base</p>
                    <p class="text-3xl font-bold">{stats.totalEntities}</p>
                </div>
                <div class="bg-purple-600 p-3 rounded-full">
                    <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
                        <circle cx="9" cy="7" r="4"/>
                        <path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
                        <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
                    </svg>
                </div>
            </div>
        </div>
        
        <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <div class="flex items-center justify-between">
                <div>
                    <p class="text-sm text-gray-400">Dernière mise à jour</p>
                    <p class="text-lg font-bold">{stats.lastUpdate}</p>
                </div>
                <div class="bg-yellow-600 p-3 rounded-full">
                    <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10"/>
                        <polyline points="12 6 12 12 16 14"/>
                    </svg>
                </div>
            </div>
        </div>
    </div>
    
    <!-- Statut des services -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <h2 class="text-xl font-semibold mb-4">Statut des services</h2>
            <div class="space-y-3">
                <div class="flex items-center justify-between">
                    <span>Connexion TOR</span>
                    <div class="flex items-center space-x-2">
                        <span class:connected={torStatus === 'Connecté'} class:disconnected={torStatus !== 'Connecté'}>
                            {torStatus}
                        </span>
                        <div class="w-3 h-3 rounded-full bg-green-500" class:offline={torStatus !== 'Connecté'}></div>
                    </div>
                </div>
                <div class="flex items-center justify-between">
                    <span>Base de données</span>
                    <div class="flex items-center space-x-2">
                        <span>{dbStatus}</span>
                        <div class="w-3 h-3 rounded-full bg-green-500"></div>
                    </div>
                </div>
            </div>
        </div>
        
        <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <h2 class="text-xl font-semibold mb-4">Activité récente</h2>
            <div class="space-y-3">
                {#each recentActivity as activity}
                    <div class="flex items-center space-x-3 p-2 bg-gray-700 rounded">
                        <div class="w-2 h-2 bg-blue-500 rounded-full"></div>
                        <div class="flex-1">
                            <p class="font-medium">{activity.description}</p>
                            <p class="text-xs text-gray-400">{activity.timestamp}</p>
                        </div>
                        <span class="text-xs px-2 py-1 bg-gray-600 rounded">{activity.type}</span>
                    </div>
                {/each}
            </div>
        </div>
    </div>
    
    <!-- Accès rapide -->
    <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
        <h2 class="text-xl font-semibold mb-4">Accès rapide</h2>
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <Link to="/acquerir" class="flex flex-col items-center p-4 bg-gray-700 rounded hover:bg-gray-600 transition-colors">
                <svg class="w-8 h-8 mb-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                    <polyline points="7 10 12 15 17 10"/>
                    <line x1="12" y1="15" x2="12" y2="3"/>
                </svg>
                <span>Importer un fichier</span>
            </Link>
            
            <Link to="/acquerir" class="flex flex-col items-center p-4 bg-gray-700 rounded hover:bg-gray-600 transition-colors">
                <svg class="w-8 h-8 mb-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="11" cy="11" r="8"/>
                    <line x1="21" y1="21" x2="16.65" y2="16.65"/>
                </svg>
                <span>Nouvelle veille</span>
            </Link>
            
            <Link to="/capitaliser" class="flex flex-col items-center p-4 bg-gray-700 rounded hover:bg-gray-600 transition-colors">
                <svg class="w-8 h-8 mb-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                    <polyline points="14 2 14 8 20 8"/>
                </svg>
                <span>Capitaliser</span>
            </Link>
            
            <Link to="/exploiter" class="flex flex-col items-center p-4 bg-gray-700 rounded hover:bg-gray-600 transition-colors">
                <svg class="w-8 h-8 mb-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polygon points="12 2 2 7 12 12 22 7 12 2"/>
                    <polyline points="2 17 12 22 22 17"/>
                    <polyline points="2 12 12 17 22 12"/>
                </svg>
                <span>Exploiter</span>
            </Link>
        </div>
    </div>
</div>

<style>
    .connected {
        color: #10b981;
    }
    .disconnected {
        color: #ef4444;
    }
    .offline {
        background-color: #6b7280 !important;
    }
</style>
