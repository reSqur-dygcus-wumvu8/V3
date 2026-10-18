<script lang="ts">
    import { invoke } from '@tauri-apps/api/tauri';
    import { onMount } from 'svelte';
    import type { DiscoveryResult } from '$lib/api';
    
    let topic = '';
    let isLoading = false;
    let result: DiscoveryResult | null = null;
    let error: string | null = null;
    
    // Options de configuration
    let selectedLanguage = 'fr';
    let includeSources = true;
    let includeKeywords = true;
    let maxTokens = 2000;
    let temperature = 0.7;
    
    const languageOptions = [
        { code: 'fr', label: 'Français' },
        { code: 'en', label: 'Anglais' },
        { code: 'es', label: 'Espagnol' },
        { code: 'de', label: 'Allemand' },
        { code: 'ru', label: 'Russe' },
    ];
    
    const temperatureOptions = [
        { value: 0.1, label: 'Très déterministe' },
        { value: 0.3, label: 'Déterministe' },
        { value: 0.5, label: 'Équilibré' },
        { value: 0.7, label: 'Créatif' },
        { value: 0.9, label: 'Très créatif' },
    ];
    
    const tokenOptions = [
        { value: 500, label: '500 tokens' },
        { value: 1000, label: '1000 tokens' },
        { value: 2000, label: '2000 tokens' },
        { value: 4000, label: '4000 tokens' },
    ];
    
    // Historique des recherches
    let searchHistory: string[] = [];
    
    onMount(() => {
        // Charger l'historique depuis localStorage
        const savedHistory = localStorage.getItem('discoveryHistory');
        if (savedHistory) {
            searchHistory = JSON.parse(savedHistory);
        }
    });
    
    async function discover() {
        if (!topic.trim()) {
            error = 'Veuillez saisir une thématique';
            return;
        }
        
        isLoading = true;
        error = null;
        result = null;
        
        try {
            // Ajouter à l'historique
            if (!searchHistory.includes(topic)) {
                searchHistory.unshift(topic);
                if (searchHistory.length > 10) {
                    searchHistory = searchHistory.slice(0, 10);
                }
                localStorage.setItem('discoveryHistory', JSON.stringify(searchHistory));
            }
            
            const response = await invoke('discover_topic', {
                topic,
                language: selectedLanguage,
                maxTokens,
                temperature,
            }) as DiscoveryResult;
            
            result = response;
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur lors de la découverte';
            console.error('Erreur:', err);
        } finally {
            isLoading = false;
        }
    }
    
    function addSourceToWatch(source: { name: string; url: string }) {
        // Cette fonction sera implémentée pour ajouter une source à une veille
        alert(`Ajouter la source: ${source.name} (${source.url})`);
    }
    
    function addKeywordToWatch(keyword: string) {
        // Cette fonction sera implémentée pour ajouter un mot-clé à une veille
        alert(`Ajouter le mot-clé: ${keyword}`);
    }
    
    function clearResults() {
        result = null;
        topic = '';
    }
</script>

<div class="space-y-6">
    <div>
        <h2 class="text-2xl font-bold">Mode Découverte</h2>
        <p class="text-gray-400">Explorez une thématique en profondeur avec Mistral</p>
    </div>
    
    <!-- Erreur -->
    {#if error}
        <div class="bg-red-900/30 border border-red-600 rounded-lg p-4">
            <p class="text-red-400">{error}</p>
            <button on:click={() => error = null} class="mt-2 text-red-400 hover:text-red-300">
                Fermer
            </button>
        </div>
    {/if}
    
    <!-- Formulaire de recherche -->
    <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 items-end">
            <div class="md:col-span-3">
                <label class="block text-sm font-medium mb-2">Thématique à explorer</label>
                <div class="relative">
                    <input 
                        type="text" 
                        bind:value={topic}
                        placeholder="Ex: Conflits en Afrique de l'Ouest, Cybermenaces 2024, etc."
                        class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none pr-10"
                        on:keydown={e => e.key === 'Enter' && !isLoading && discover()}
                    />
                    {#if searchHistory.length > 0 && !topic}
                        <div class="absolute top-full left-0 right-0 mt-1 bg-gray-700 rounded border border-gray-600 shadow-lg z-10">
                            <div class="p-2">
                                <p class="text-xs text-gray-400 mb-1">Historique:</p>
                                {#each searchHistory.slice(0, 5) as historyItem}
                                    <button 
                                        on:click={() => { topic = historyItem; discover(); }}
                                        class="block w-full text-left p-1 rounded hover:bg-gray-600 text-sm truncate"
                                    >
                                        {historyItem}
                                    </button>
                                {/each}
                            </div>
                        </div>
                    {/if}
                </div>
            </div>
            
            <div>
                <button 
                    on:click={discover}
                    disabled={!topic.trim() || isLoading}
                    class="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed px-4 py-2 rounded transition-colors font-medium flex items-center justify-center space-x-2"
                >
                    {#if isLoading}
                        <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                        <span>Exploration...</span>
                    {:else}
                        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="10"/>
                            <line x1="12" y1="8" x2="12" y2="16"/>
                            <line x1="8" y1="12" x2="16" y2="12"/>
                        </svg>
                        <span>Explorer</span>
                    {/if}
                </button>
            </div>
        </div>
        
        <!-- Options avancées -->
        <div class="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
                <label class="block text-sm font-medium mb-2">Langue</label>
                <select 
                    bind:value={selectedLanguage}
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                >
                    {#each languageOptions as option}
                        <option value={option.code}>{option.label}</option>
                    {/each}
                </select>
            </div>
            
            <div>
                <label class="block text-sm font-medium mb-2">Température</label>
                <select 
                    bind:value={temperature}
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                >
                    {#each temperatureOptions as option}
                        <option value={option.value}>{option.label}</option>
                    {/each}
                </select>
            </div>
            
            <div>
                <label class="block text-sm font-medium mb-2">Tokens max</label>
                <select 
                    bind:value={maxTokens}
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                >
                    {#each tokenOptions as option}
                        <option value={option.value}>{option.label}</option>
                    {/each}
                </select>
            </div>
            
            <div class="flex items-center">
                <label class="flex items-center space-x-2 cursor-pointer">
                    <input type="checkbox" bind:checked={includeSources} class="h-4 w-4 text-blue-600" />
                    <span class="text-sm">Inclure les sources</span>
                </label>
            </div>
        </div>
    </div>
    
    <!-- Résultats -->
    {#if result}
        <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <div class="flex items-center justify-between mb-4">
                <h3 class="text-xl font-semibold">Résultats pour "{topic}"</h3>
                <button on:click={clearResults} class="text-gray-400 hover:text-white">
                    <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10"/>
                        <line x1="15" y1="9" x2="9" y2="15"/>
                        <line x1="9" y1="9" x2="15" y2="15"/>
                    </svg>
                </button>
            </div>
            
            <!-- Synthèse -->
            <div class="mb-6">
                <h4 class="text-lg font-medium mb-2">Synthèse structurée</h4>
                <div class="bg-gray-700 p-4 rounded prose max-w-none">
                    {@html result.summary}
                </div>
            </div>
            
            <!-- Sources proposées -->
            {#if result.sources && result.sources.length > 0 && includeSources}
                <div class="mb-6">
                    <h4 class="text-lg font-medium mb-2">Sources proposées ({result.sources.length})</h4>
                    <div class="space-y-2">
                        {#each result.sources as source, i}
                            <div class="flex items-center justify-between p-3 bg-gray-700 rounded">
                                <div>
                                    <a 
                                        href={source.url} 
                                        target="_blank" 
                                        rel="noopener noreferrer"
                                        class="text-blue-400 hover:text-blue-300 font-medium"
                                    >
                                        {source.name}
                                    </a>
                                    <p class="text-sm text-gray-400 truncate">{source.url}</p>
                                </div>
                                <button 
                                    on:click={() => addSourceToWatch(source)}
                                    class="bg-gray-600 hover:bg-gray-500 px-3 py-1 rounded text-sm transition-colors"
                                >
                                    + Ajouter à une veille
                                </button>
                            </div>
                        {/each}
                    </div>
                </div>
            {/if}
            
            <!-- Mots-clés -->
            {#if result.keywords && result.keywords.length > 0 && includeKeywords}
                <div>
                    <h4 class="text-lg font-medium mb-2">Mots-clés pour les veilles ({result.keywords.length})</h4>
                    <div class="flex flex-wrap gap-2">
                        {#each result.keywords as keyword}
                            <button 
                                on:click={() => addKeywordToWatch(keyword)}
                                class="bg-gray-700 hover:bg-gray-600 px-3 py-1 rounded text-sm transition-colors flex items-center space-x-1"
                            >
                                <span>{keyword}</span>
                                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <polyline points="20 6 9 17 4 12"/>
                                </svg>
                            </button>
                        {/each}
                    </div>
                </div>
            {/if}
        </div>
    {/if}
    
    <!-- Conseils -->
    <div class="bg-gray-800/50 rounded-lg p-6 border border-gray-700/50">
        <h3 class="text-lg font-medium mb-3">Conseils pour le mode découverte</h3>
        <ul class="list-disc list-inside space-y-2 text-gray-400">
            <li>Soyez précis dans votre thématique pour obtenir des résultats plus pertinents</li>
            <li>Utilisez des mots-clés spécifiques plutôt que des termes génériques</li>
            <li>Le mode découverte utilise le modèle Mistral en mode réflexion pour une analyse approfondie</li>
            <li>Les résultats sont automatiquement traduits en français si nécessaire</li>
            <li>Vous pouvez ajouter les sources et mots-clés proposés à vos veilles existantes</li>
        </ul>
    </div>
</div>
