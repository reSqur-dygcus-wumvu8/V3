<script lang="ts">
    import { Link, Route, Router } from 'svelte-routing';
    import ImportFile from './ImportFile.svelte';
    import WatchConfig from './WatchConfig.svelte';
    import DiscoveryMode from './DiscoveryMode.svelte';
    
    // Sous-pages
    const routes = {
        '/acquerir': ImportFile,
        '/acquerir/importer': ImportFile,
        '/acquerir/veille': WatchConfig,
        '/acquerir/decouverte': DiscoveryMode,
    };
</script>

<div class="space-y-6">
    <div>
        <h1 class="text-3xl font-bold">Acquérir</h1>
        <p class="text-gray-400">Collecte d'informations depuis diverses sources</p>
    </div>
    
    <!-- Navigation interne -->
    <nav class="bg-gray-800 rounded-lg p-4 border border-gray-700">
        <div class="flex space-x-4">
            <Link 
                to="/acquerir/importer" 
                class="flex items-center space-x-2 px-4 py-2 rounded hover:bg-gray-700 transition-colors"
                class:active="bg-blue-600"
            >
                <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                    <polyline points="7 10 12 15 17 10"/>
                    <line x1="12" y1="15" x2="12" y2="3"/>
                </svg>
                <span>Importer des fichiers</span>
            </Link>
            
            <Link 
                to="/acquerir/veille" 
                class="flex items-center space-x-2 px-4 py-2 rounded hover:bg-gray-700 transition-colors"
                class:active="bg-blue-600"
            >
                <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10"/>
                    <polyline points="12 6 12 12 12 18"/>
                    <line x1="6" y1="12" x2="18" y2="12"/>
                </svg>
                <span>Veille Internet</span>
            </Link>
            
            <Link 
                to="/acquerir/decouverte" 
                class="flex items-center space-x-2 px-4 py-2 rounded hover:bg-gray-700 transition-colors"
                class:active="bg-blue-600"
            >
                <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10"/>
                    <line x1="12" y1="8" x2="12" y2="16"/>
                    <line x1="8" y1="12" x2="16" y2="12"/>
                </svg>
                <span>Mode Découverte</span>
            </Link>
        </div>
    </nav>
    
    <!-- Contenu des sous-pages -->
    <div class="bg-gray-800 rounded-lg p-6 border border-gray-700 min-h-[400px]">
        <Route path="/acquerir/*" />
    </div>
</div>

<style>
    .active {
        background-color: rgb(37 99 235);
        color: white;
    }
</style>
