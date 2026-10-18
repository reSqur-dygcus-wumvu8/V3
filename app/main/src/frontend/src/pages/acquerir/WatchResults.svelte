<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/tauri';
    import { formatDate, getReliabilityLabel, getCredibilityLabel, getReliabilityColor, getCredibilityColor } from '$lib/utils';
    import type { WatchResult, ProcessedWatchResult, EventGroup } from '$types';
    
    let isLoading = true;
    let error: string | null = null;
    let processedResults: ProcessedWatchResult[] = [];
    let eventGroups: EventGroup[] = [];
    let selectedEvent: EventGroup | null = null;
    let selectedResult: ProcessedWatchResult | null = null;
    
    // Filtres
    let filterSource = 'all';
    let filterRating = 'all';
    let searchQuery = '';
    
    onMount(async () => {
        await loadResults();
    });
    
    async function loadResults() {
        isLoading = true;
        error = null;
        
        try {
            // Récupérer les résultats traités
            processedResults = await invoke('get_processed_watch_results') as ProcessedWatchResult[];
            
            // Regrouper par événement
            eventGroups = groupByEvent(processedResults);
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur lors du chargement des résultats';
        } finally {
            isLoading = false;
        }
    }
    
    function groupByEvent(results: ProcessedWatchResult[]): EventGroup[] {
        const groups: Record<string, EventGroup> = {};
        
        for (const result of results) {
            // Utiliser le titre comme clé d'événement (simplifié)
            const eventKey = result.title;
            
            if (!groups[eventKey]) {
                groups[eventKey] = {
                    event: eventKey,
                    date: result.published_at || result.fetched_at,
                    results: [],
                    proposedRating: result.proposed_rating,
                };
            }
            
            groups[eventKey].results.push(result);
        }
        
        return Object.values(groups);
    }
    
    function getUniqueSources() {
        const sources = new Set<string>();
        for (const result of processedResults) {
            sources.add(result.source_name);
        }
        return Array.from(sources).sort();
    }
    
    function getFilteredResults() {
        let results = processedResults;
        
        // Filtrer par source
        if (filterSource !== 'all') {
            results = results.filter(r => r.source_name === filterSource);
        }
        
        // Filtrer par cotation
        if (filterRating !== 'all') {
            results = results.filter(r => r.proposed_rating === filterRating);
        }
        
        // Filtrer par recherche
        if (searchQuery) {
            const query = searchQuery.toLowerCase();
            results = results.filter(r => 
                r.title.toLowerCase().includes(query) ||
                r.content.toLowerCase().includes(query) ||
                r.source_name.toLowerCase().includes(query)
            );
        }
        
        return results;
    }
    
    function getFilteredEventGroups() {
        const filteredResults = getFilteredResults();
        return groupByEvent(filteredResults);
    }
    
    async function processResults() {
        try {
            await invoke('process_watch_results');
            await loadResults();
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur lors du traitement des résultats';
        }
    }
    
    function selectEvent(event: EventGroup) {
        selectedEvent = event;
        selectedResult = null;
    }
    
    function selectResult(result: ProcessedWatchResult) {
        selectedResult = result;
        selectedEvent = null;
    }
    
    function closeModal() {
        selectedEvent = null;
        selectedResult = null;
    }
    
    async function capitalizeResult(result: ProcessedWatchResult) {
        try {
            // Appeler la fonction de capitalisation
            await invoke('capitalize_info', {
                watchResultId: result.id,
                entityType: 'event', // À adapter
                entityId: 0, // À adapter
                cotation: result.proposed_rating,
            });
            
            // Marquer comme traité (simulé)
            await loadResults();
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur lors de la capitalisation';
        }
    }
</script>

<div class="space-y-6">
    <div class="flex items-center justify-between">
        <div>
            <h2 class="text-2xl font-bold">Résultats des veilles</h2>
            <p class="text-gray-400">Résultats prêts à être capitalisés</p>
        </div>
        
        <div class="flex space-x-2">
            <button 
                on:click={loadResults}
                class="bg-gray-700 hover:bg-gray-600 px-3 py-2 rounded transition-colors text-sm"
            >
                <svg class="w-4 h-4 inline mr-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="23 4 23 10 17 10"/>
                    <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
                </svg>
                Rafraîchir
            </button>
            
            <button 
                on:click={processResults}
                class="bg-blue-600 hover:bg-blue-700 px-3 py-2 rounded transition-colors text-sm"
            >
                <svg class="w-4 h-4 inline mr-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polygon points="22 12 18 12 15 21 9 3 6 12 2 12"/>
                </svg>
                Traiter les résultats
            </button>
        </div>
    </div>
    
    <!-- Erreur -->
    {#if error}
        <div class="bg-red-900/30 border border-red-600 rounded-lg p-4">
            <p class="text-red-400">{error}</p>
            <button on:click={() => error = null} class="mt-2 text-red-400 hover:text-red-300 text-sm">
                Fermer
            </button>
        </div>
    {/if}
    
    <!-- Filtres -->
    <div class="bg-gray-800 rounded-lg p-4 border border-gray-700">
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
                <label class="block text-sm font-medium mb-2">Source</label>
                <select 
                    bind:value={filterSource}
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                >
                    <option value="all">Toutes les sources</option>
                    {#each getUniqueSources() as source}
                        <option value={source}>{source}</option>
                    {/each}
                </select>
            </div>
            
            <div>
                <label class="block text-sm font-medium mb-2">Cotation</label>
                <select 
                    bind:value={filterRating}
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                >
                    <option value="all">Toutes les cotations</option>
                    <option value="A1">A1 - Totalement fiable / Confirmée</option>
                    <option value="A2">A2 - Totalement fiable / Probablement vraie</option>
                    <option value="A3">A3 - Totalement fiable / Possiblement vraie</option>
                    <option value="B1">B1 - Généralement fiable / Confirmée</option>
                    <option value="B2">B2 - Généralement fiable / Probablement vraie</option>
                    <option value="B3">B3 - Généralement fiable / Possiblement vraie</option>
                    <option value="C1">C1 - Assez fiable / Confirmée</option>
                    <option value="C2">C2 - Assez fiable / Probablement vraie</option>
                    <option value="D4">D4 - Peu fiable / Douteuse</option>
                    <option value="E5">E5 - Fiabilité non évaluable / Improbable</option>
                    <option value="F6">F6 - Non évaluable / Non évaluable</option>
                </select>
            </div>
            
            <div>
                <label class="block text-sm font-medium mb-2">Recherche</label>
                <input 
                    type="text" 
                    bind:value={searchQuery}
                    placeholder="Rechercher dans les résultats..."
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                />
            </div>
        </div>
    </div>
    
    <!-- Résultats -->
    {#if isLoading}
        <div class="flex items-center justify-center p-8">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
            <p class="ml-4">Chargement des résultats...</p>
        </div>
    {:else if getFilteredEventGroups().length === 0}
        <div class="bg-gray-800 rounded-lg p-8 border border-gray-700 text-center">
            <svg class="w-12 h-12 mx-auto mb-4 text-gray-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <line x1="12" y1="8" x2="12" y2="16"/>
                <line x1="8" y1="12" x2="16" y2="12"/>
            </svg>
            <p class="text-gray-400">Aucun résultat de veille à afficher</p>
            <p class="text-gray-500 text-sm mt-2">Exécutez une veille ou traitez les résultats existants</p>
        </div>
    {:else}
        <div class="space-y-4">
            {#each getFilteredEventGroups() as group}
                <div class="bg-gray-800 rounded-lg p-4 border border-gray-700 hover:bg-gray-700/50 transition-colors">
                    <div class="flex items-start justify-between">
                        <div class="flex-1">
                            <div class="flex items-center space-x-2 mb-2">
                                <h3 class="text-lg font-semibold">{group.event}</h3>
                                <span 
                                    class="text-xs px-2 py-1 rounded {getReliabilityColor(group.proposedRating[0])} {getCredibilityColor(group.proposedRating[1])}"
                                    title="Cotation proposée: {group.proposedRating}"
                                >
                                    {group.proposedRating}
                                </span>
                            </div>
                            
                            {#if group.date}
                                <p class="text-sm text-gray-400 mb-2">
                                    {formatDate(group.date)}
                                </p>
                            {/if}
                            
                            <div class="flex flex-wrap gap-2 mb-2">
                                {#each group.results as result}
                                    <span class="bg-gray-700 px-2 py-1 rounded text-xs">
                                        {result.source_name}
                                    </span>
                                {/each}
                            </div>
                            
                            <p class="text-sm text-gray-400">
                                {group.results.length} résultat(s) pour cet événement
                            </p>
                        </div>
                        
                        <div class="flex space-x-2">
                            <button 
                                on:click={() => selectEvent(group)}
                                class="bg-blue-600 hover:bg-blue-700 px-3 py-1 rounded transition-colors text-sm"
                            >
                                Voir les détails
                            </button>
                            
                            <button 
                                on:click={() => capitalizeResult(group.results[0])}
                                class="bg-green-600 hover:bg-green-700 px-3 py-1 rounded transition-colors text-sm"
                            >
                                Capitaliser
                            </button>
                        </div>
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</div>

<!-- Modale : Détails d'un événement -->
{#if selectedEvent}
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
        <div class="bg-gray-900 rounded-lg w-full max-w-4xl max-h-[90vh] overflow-y-auto p-6">
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-2xl font-bold">{selectedEvent.event}</h2>
                <button on:click={closeModal} class="text-gray-400 hover:text-white">
                    <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="18" y1="6" x2="6" y2="18"/>
                        <line x1="6" y1="6" x2="18" y2="18"/>
                    </svg>
                </button>
            </div>
            
            <div class="mb-4">
                <p><strong>Date:</strong> {formatDate(selectedEvent.date || '')}</p>
                <p><strong>Cotation proposée:</strong> {selectedEvent.proposedRating}</p>
                <p class="text-sm text-gray-400">
                    {getReliabilityLabel(selectedEvent.proposedRating[0])} / {getCredibilityLabel(selectedEvent.proposedRating[1])}
                </p>
            </div>
            
            <div class="space-y-4">
                {#each selectedEvent.results as result}
                    <div class="bg-gray-800 rounded p-4 border border-gray-700">
                        <div class="flex items-start justify-between mb-2">
                            <div class="flex-1">
                                <h3 class="font-semibold mb-1">{result.title}</h3>
                                <p class="text-sm text-gray-400 mb-2">
                                    Source: {result.source_name} | {formatDate(result.published_at || result.fetched_at || '')}
                                </p>
                            </div>
                            <span 
                                class="text-xs px-2 py-1 rounded {getReliabilityColor(result.proposed_rating[0])} {getCredibilityColor(result.proposed_rating[1])} whitespace-nowrap"
                            >
                                {result.proposed_rating}
                            </span>
                        </div>
                        
                        <p class="text-gray-300 mb-2">{result.content}</p>
                        
                        {#if result.translated_content}
                            <details class="mb-2">
                                <summary class="text-sm text-gray-400 cursor-pointer">
                                    Traduction en français
                                </summary>
                                <p class="text-gray-300 mt-2">{result.translated_content}</p>
                            </details>
                        {/if}
                        
                        {#if result.entities && result.entities.length > 0}
                            <div class="mt-2">
                                <p class="text-sm text-gray-400 mb-1">Entités détectées:</p>
                                <div class="flex flex-wrap gap-1">
                                    {#each result.entities as entity}
                                        <span 
                                            class="bg-gray-700 px-2 py-1 rounded text-xs {entity.matched ? 'border border-green-500' : ''}"
                                            title="Type: {entity.entity_type}, Confiance: {entity.confidence}"
                                        >
                                            {entity.name}
                                        </span>
                                    {/each}
                                </div>
                            </div>
                        {/if}
                        
                        <div class="flex space-x-2 mt-3">
                            <button 
                                on:click={() => selectResult(result)}
                                class="bg-gray-600 hover:bg-gray-500 px-3 py-1 rounded transition-colors text-sm"
                            >
                                Voir plus
                            </button>
                            
                            <button 
                                on:click={() => capitalizeResult(result)}
                                class="bg-green-600 hover:bg-green-700 px-3 py-1 rounded transition-colors text-sm"
                            >
                                Capitaliser
                            </button>
                        </div>
                    </div>
                {/each}
            </div>
        </div>
    </div>
{/if}

<!-- Modale : Détails d'un résultat -->
{#if selectedResult}
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
        <div class="bg-gray-900 rounded-lg w-full max-w-3xl max-h-[90vh] overflow-y-auto p-6">
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-xl font-bold">{selectedResult.title}</h2>
                <button on:click={closeModal} class="text-gray-400 hover:text-white">
                    <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="18" y1="6" x2="6" y2="18"/>
                        <line x1="6" y1="6" x2="18" y2="18"/>
                    </svg>
                </button>
            </div>
            
            <div class="mb-4">
                <p><strong>Source:</strong> {selectedResult.source_name}</p>
                <p><strong>Date:</strong> {formatDate(selectedResult.published_at || selectedResult.fetched_at || '')}</p>
                <p><strong>Cotation:</strong> {selectedResult.proposed_rating}</p>
                <p class="text-sm text-gray-400">
                    Fiabilité: {getReliabilityLabel(selectedResult.source_rating)} | 
                    Crédibilité: {getCredibilityLabel(selectedResult.info_rating)}
                </p>
            </div>
            
            <div class="mb-4">
                <h3 class="font-semibold mb-2">Contenu:</h3>
                <p class="text-gray-300 whitespace-pre-wrap">{selectedResult.content}</p>
            </div>
            
            {#if selectedResult.translated_content}
                <div class="mb-4">
                    <h3 class="font-semibold mb-2">Traduction:</h3>
                    <p class="text-gray-300 whitespace-pre-wrap">{selectedResult.translated_content}</p>
                </div>
            {/if}
            
            {#if selectedResult.entities && selectedResult.entities.length > 0}
                <div class="mb-4">
                    <h3 class="font-semibold mb-2">Entités détectées ({selectedResult.entities.length}):</h3>
                    <div class="space-y-2">
                        {#each selectedResult.entities as entity}
                            <div class="flex items-center justify-between p-2 bg-gray-800 rounded">
                                <div>
                                    <span class="font-medium">{entity.name}</span>
                                    <span class="text-sm text-gray-400 ml-2">({entity.entity_type})</span>
                                    <span class="text-xs text-gray-500 ml-2">Confiance: {(entity.confidence * 100).toFixed(1)}%</span>
                                </div>
                                <div>
                                    {#if entity.matched}
                                        <span class="text-green-400 text-sm">✓ Correspondance trouvée</span>
                                    {:else}
                                        <button class="bg-blue-600 hover:bg-blue-700 px-2 py-1 rounded transition-colors text-xs">
                                            Créer une fiche
                                        </button>
                                    {/if}
                                </div>
                            </div>
                        {/each}
                    </div>
                </div>
            {/if}
            
            <div class="flex justify-end space-x-2">
                <button 
                    on:click={closeModal}
                    class="bg-gray-600 hover:bg-gray-500 px-4 py-2 rounded transition-colors"
                >
                    Fermer
                </button>
                
                <button 
                    on:click={() => capitalizeResult(selectedResult)}
                    class="bg-green-600 hover:bg-green-700 px-4 py-2 rounded transition-colors"
                >
                    Capitaliser ce résultat
                </button>
            </div>
        </div>
    </div>
{/if}
