<script lang="ts">
    import { invoke } from '@tauri-apps/api/tauri';
    import { onMount } from 'svelte';
    import { calculateSHA256 } from '$lib/utils';
    import type { Watch } from '$types';
    
    let files: FileList | null = null;
    let isDragging = false;
    let isProcessing = false;
    let uploadProgress = 0;
    let totalFiles = 0;
    let processedFiles = 0;
    let importResults: Array<{name: string; status: 'success' | 'error' | 'duplicate'; message?: string}> = [];
    
    // Cotation de la source
    let sourceRating = 'B';
    const reliabilityOptions = [
        { code: 'A', label: 'Totalement fiable' },
        { code: 'B', label: 'Généralement fiable' },
        { code: 'C', label: 'Assez fiable' },
        { code: 'D', label: 'Peu fiable' },
        { code: 'E', label: 'Fiabilité non évaluable' },
        { code: 'F', label: 'Non évaluable' },
    ];
    
    // Source sélectionnée
    let selectedSource = 'Import local';
    let customSourceName = '';
    
    // Veilles existantes pour lier les imports
    let watches: Watch[] = [];
    let selectedWatchId: number | null = null;
    
    onMount(async () => {
        try {
            watches = await invoke('get_watches') as Watch[];
        } catch (error) {
            console.error('Erreur lors du chargement des veilles:', error);
        }
    });
    
    function handleDragEnter(e: DragEvent) {
        e.preventDefault();
        isDragging = true;
    }
    
    function handleDragLeave(e: DragEvent) {
        e.preventDefault();
        isDragging = false;
    }
    
    function handleDragOver(e: DragEvent) {
        e.preventDefault();
    }
    
    function handleDrop(e: DragEvent) {
        e.preventDefault();
        isDragging = false;
        if (e.dataTransfer?.files) {
            files = e.dataTransfer.files;
        }
    }
    
    function handleFileChange(e: Event) {
        const input = e.target as HTMLInputElement;
        if (input.files) {
            files = input.files;
        }
    }
    
    async function processFiles() {
        if (!files || files.length === 0) return;
        
        isProcessing = true;
        totalFiles = files.length;
        processedFiles = 0;
        importResults = [];
        uploadProgress = 0;
        
        // Créer ou obtenir la source
        const sourceName = customSourceName || selectedSource;
        let sourceId: number;
        
        try {
            // Vérifier si la source existe déjà
            const existingSource = await invoke('get_source_by_name', { name: sourceName });
            sourceId = existingSource as number;
            
            if (!sourceId) {
                // Créer une nouvelle source
                sourceId = await invoke('create_source', { 
                    name: sourceName, 
                    description: `Source: ${selectedSource}`,
                    reliability: sourceRating,
                }) as number;
            }
        } catch (error) {
            console.error('Erreur lors de la création de la source:', error);
            isProcessing = false;
            return;
        }
        
        // Traiter chaque fichier
        for (let i = 0; i < files.length; i++) {
            const file = files[i];
            
            try {
                // Calculer le hash pour la déduplication
                const hash = await calculateSHA256(file);
                
                // Vérifier si le fichier existe déjà
                const isDuplicate = await invoke('check_file_duplicate', { hash }) as boolean;
                
                if (isDuplicate) {
                    importResults.push({
                        name: file.name,
                        status: 'duplicate',
                        message: 'Fichier déjà importé'
                    });
                    processedFiles++;
                    uploadProgress = Math.round((processedFiles / totalFiles) * 100);
                    continue;
                }
                
                // Lire le contenu du fichier
                const arrayBuffer = await file.arrayBuffer();
                const uint8Array = new Uint8Array(arrayBuffer);
                
                // Importer le fichier
                await invoke('import_file', {
                    name: file.name,
                    data: Array.from(uint8Array),
                    sourceId,
                    sourceRating,
                    date: new Date().toISOString(),
                    hash,
                    watchId: selectedWatchId,
                });
                
                importResults.push({
                    name: file.name,
                    status: 'success',
                });
                
                // Lancer le traitement NER en arrière-plan
                if (selectedWatchId) {
                    await invoke('process_file_with_ner', {
                        fileId: processedFiles + 1,
                        watchId: selectedWatchId,
                    });
                }
            } catch (error) {
                console.error(`Erreur lors de l'import du fichier ${file.name}:`, error);
                importResults.push({
                    name: file.name,
                    status: 'error',
                    message: error instanceof Error ? error.message : 'Erreur inconnue'
                });
            }
            
            processedFiles++;
            uploadProgress = Math.round((processedFiles / totalFiles) * 100);
        }
        
        isProcessing = false;
    }
    
    function clearSelection() {
        files = null;
        importResults = [];
        uploadProgress = 0;
        processedFiles = 0;
        totalFiles = 0;
    }
</script>

<div class="space-y-6">
    <div>
        <h2 class="text-2xl font-bold">Importer des fichiers</h2>
        <p class="text-gray-400">Importez des fichiers depuis votre système local ou depuis une URL</p>
    </div>
    
    <!-- Zone de dépôt -->
    <div 
        class="border-2 border-dashed border-gray-600 rounded-lg p-8 text-center transition-all"
        class:dragging="border-blue-500 bg-gray-800"
        on:dragenter={handleDragEnter}
        on:dragleave={handleDragLeave}
        on:dragover={handleDragOver}
        on:drop={handleDrop}
    >
        <input 
            type="file" 
            multiple 
            on:change={handleFileChange}
            class="hidden" 
            id="file-input"
        />
        
        {#if !files || files.length === 0}
            <label for="file-input" class="cursor-pointer">
                <svg class="w-12 h-12 mx-auto mb-4 text-gray-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                    <polyline points="7 10 12 15 17 10"/>
                    <line x1="12" y1="15" x2="12" y2="3"/>
                </svg>
                <p class="text-lg font-medium">Glissez-déposez vos fichiers ici</p>
                <p class="text-gray-400 text-sm mb-4">ou cliquez pour parcourir</p>
                <button class="bg-blue-600 hover:bg-blue-700 px-4 py-2 rounded transition-colors">
                    Sélectionner des fichiers
                </button>
            </label>
        {:else}
            <div class="space-y-4">
                <p class="text-lg font-medium">Fichiers sélectionnés: {files.length}</p>
                <ul class="text-left max-h-40 overflow-y-auto">
                    {#each Array.from(files) as file}
                        <li class="flex items-center justify-between p-2 bg-gray-800 rounded">
                            <span class="truncate max-w-xs">{file.name} ({formatFileSize(file.size)})</span>
                            <span class="text-gray-400">{file.type}</span>
                        </li>
                    {/each}
                </ul>
                <button 
                    on:click={clearSelection}
                    class="text-gray-400 hover:text-white text-sm"
                >
                    Effacer la sélection
                </button>
            </div>
        {/if}
    </div>
    
    <!-- Paramètres d'import -->
    <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
        <h3 class="text-xl font-semibold mb-4">Paramètres d'import</h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <!-- Source -->
            <div>
                <label class="block text-sm font-medium mb-2">Source</label>
                <select 
                    bind:value={selectedSource} 
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                >
                    <option value="Import local">Import local</option>
                    <option value="Téléchargement">Téléchargement depuis Internet</option>
                    <option value="Autre">Autre...</option>
                </select>
                {#if selectedSource === 'Autre'}
                    <input 
                        type="text" 
                        bind:value={customSourceName} 
                        placeholder="Nom de la source" 
                        class="w-full p-2 mt-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                    />
                {/if}
            </div>
            
            <!-- Cotation de la source -->
            <div>
                <label class="block text-sm font-medium mb-2">Cotation de la source (Fiabilité)</label>
                <select 
                    bind:value={sourceRating} 
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                >
                    {#each reliabilityOptions as option}
                        <option value={option.code}>{option.code} - {option.label}</option>
                    {/each}
                </select>
            </div>
            
            <!-- Veille associée -->
            <div class="md:col-span-2">
                <label class="block text-sm font-medium mb-2">Lier à une veille existante (optionnel)</label>
                <select 
                    bind:value={selectedWatchId} 
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                >
                    <option value={null}>Aucune veille</option>
                    {#each watches as watch}
                        <option value={watch.id}>{watch.name} ({watch.query})</option>
                    {/each}
                </select>
            </div>
        </div>
        
        <!-- Bouton d'import -->
        <div class="mt-6">
            {#if isProcessing}
                <div class="w-full bg-gray-700 rounded-full h-2">
                    <div 
                        class="bg-blue-600 h-2 rounded-full transition-all"
                        style={`width: ${uploadProgress}%`}
                    ></div>
                </div>
                <p class="text-sm text-gray-400 mt-2">
                    Traitement en cours... ({processedFiles}/{totalFiles} fichiers)
                </p>
            {:else}
                <button 
                    on:click={processFiles}
                    disabled={!files || files.length === 0 || isProcessing}
                    class="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed px-4 py-2 rounded transition-colors font-medium"
                >
                    Importer les fichiers
                </button>
            {/if}
        </div>
    </div>
    
    <!-- Résultats de l'import -->
    {#if importResults.length > 0}
        <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <h3 class="text-xl font-semibold mb-4">Résultats de l'import</h3>
            <div class="space-y-2">
                {#each importResults as result}
                    <div class="flex items-center justify-between p-2 rounded {result.status === 'success' ? 'bg-green-900/30' : result.status === 'error' ? 'bg-red-900/30' : 'bg-yellow-900/30'}">
                        <div class="flex items-center space-x-2">
                            {#if result.status === 'success'}
                                <svg class="w-5 h-5 text-green-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <polyline points="20 6 9 17 4 12"/>
                                </svg>
                            {:else if result.status === 'error'}
                                <svg class="w-5 h-5 text-red-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <circle cx="12" cy="12" r="10"/>
                                    <line x1="15" y1="9" x2="9" y2="15"/>
                                    <line x1="9" y1="9" x2="15" y2="15"/>
                                </svg>
                            {:else}
                                <svg class="w-5 h-5 text-yellow-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <circle cx="12" cy="12" r="10"/>
                                    <line x1="12" y1="8" x2="12" y2="12"/>
                                    <line x1="12" y1="16" x2="12.01" y2="16"/>
                                </svg>
                            {/if}
                            <span>{result.name}</span>
                        </div>
                        <div>
                            {#if result.status === 'success'}
                                <span class="text-green-500 text-sm">Importé</span>
                            {:else if result.status === 'error'}
                                <span class="text-red-500 text-sm">Erreur: {result.message || 'Inconnue'}</span>
                            {:else}
                                <span class="text-yellow-500 text-sm">{result.message || 'Doublon'}</span>
                            {/if}
                        </div>
                    </div>
                {/each}
            </div>
        </div>
    {/if}
</div>

<script lang="ts">
    function formatFileSize(bytes: number): string {
        if (bytes === 0) return '0 octets';
        const k = 1024;
        const sizes = ['octets', 'Ko', 'Mo', 'Go'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }
</script>
