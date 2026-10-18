<script lang="ts">
    import { onMount } from 'svelte';
    import { Router, Route, Link } from 'svelte-routing';
    
    // Pages
    import Home from './pages/Home.svelte';
    import Acquerir from './pages/acquerir/Acquerir.svelte';
    import Capitaliser from './pages/capitaliser/Capitaliser.svelte';
    import Exploiter from './pages/exploiter/Exploiter.svelte';
    import Gerer from './pages/gerer/Gerer.svelte';
    
    // Stores
    import { currentPage } from '$stores/navigation';
    
    let routes = {
        '/': Home,
        '/acquerir': Acquerir,
        '/capitaliser': Capitaliser,
        '/exploiter': Exploiter,
        '/gerer': Gerer,
    };
</script>

<Router {routes}>
    <div class="min-h-screen bg-gray-900 text-gray-100 flex flex-col">
        <!-- Barre de navigation -->
        <nav class="bg-gray-800 border-b border-gray-700 px-4 py-2">
            <div class="max-w-7xl mx-auto flex items-center justify-between">
                <div class="flex items-center space-x-8">
                    <Link to="/" class="flex items-center space-x-2 text-xl font-bold">
                        <svg class="w-8 h-8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M12 2L2 7l10 5 10-5-10 5z"/>
                            <path d="M2 17l10 5 10-5-10-5-10 5z"/>
                            <path d="M2 12l10 5 10-5"/>
                        </svg>
                        <span>OSINT App</span>
                    </Link>
                    
                    <div class="hidden md:flex space-x-6">
                        <Link 
                            to="/acquerir" 
                            class="flex items-center space-x-1 px-3 py-2 rounded hover:bg-gray-700 transition-colors"
                            class:active="bg-blue-600"
                        >
                            <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                                <polyline points="7 10 12 15 17 10"/>
                                <line x1="12" y1="15" x2="12" y2="3"/>
                            </svg>
                            <span>Acquérir</span>
                        </Link>
                        
                        <Link 
                            to="/capitaliser" 
                            class="flex items-center space-x-1 px-3 py-2 rounded hover:bg-gray-700 transition-colors"
                            class:active="bg-blue-600"
                        >
                            <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                                <polyline points="14 2 14 8 20 8"/>
                                <line x1="16" y1="13" x2="8" y2="13"/>
                                <line x1="16" y1="17" x2="8" y2="17"/>
                            </svg>
                            <span>Capitaliser</span>
                        </Link>
                        
                        <Link 
                            to="/exploiter" 
                            class="flex items-center space-x-1 px-3 py-2 rounded hover:bg-gray-700 transition-colors"
                            class:active="bg-blue-600"
                        >
                            <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <polygon points="12 2 2 7 12 12 22 7 12 2"/>
                                <polyline points="2 17 12 22 22 17"/>
                                <polyline points="2 12 12 17 22 12"/>
                            </svg>
                            <span>Exploiter</span>
                        </Link>
                        
                        <Link 
                            to="/gerer" 
                            class="flex items-center space-x-1 px-3 py-2 rounded hover:bg-gray-700 transition-colors"
                            class:active="bg-blue-600"
                        >
                            <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M20 7h-9"/>
                                <path d="M14 17H5"/>
                                <circle cx="17" cy="17" r="3"/>
                                <circle cx="7" cy="7" r="3"/>
                            </svg>
                            <span>Gérer</span>
                        </Link>
                    </div>
                </div>
                
                <div class="flex items-center space-x-4">
                    <!-- Statut TOR -->
                    <div class="flex items-center space-x-1">
                        <div class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                        <span class="text-sm">TOR: Connecté</span>
                    </div>
                    
                    <!-- Statut base de données -->
                    <div class="flex items-center space-x-1">
                        <div class="w-2 h-2 bg-green-500 rounded-full"></div>
                        <span class="text-sm">DB: Chiffrée</span>
                    </div>
                </div>
            </div>
        </nav>
        
        <!-- Contenu principal -->
        <main class="flex-1 max-w-7xl mx-auto w-full p-4">
            <Route path="/*" />
        </main>
        
        <!-- Pied de page -->
        <footer class="border-t border-gray-700 px-4 py-2 text-center text-sm text-gray-500">
            <p>OSINT App v0.1.0 - © 2024 Votre Organisation</p>
        </footer>
    </div>
</Router>

<style>
    .active {
        background-color: rgb(37 99 235);
        color: white;
    }
</style>
