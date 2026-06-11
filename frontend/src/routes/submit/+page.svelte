<script lang="ts">
	let title = '';
	let description = '';
	let sourceCodeFiles: FileList;
	let thumbnailFiles: FileList;
	
	let isSubmitting = false;
	let statusMessage = '';
	let isError = false;

	async function handleSubmit() {
		isSubmitting = true;
		statusMessage = '';
		isError = false;

		if (!sourceCodeFiles || sourceCodeFiles.length === 0) {
			statusMessage = 'O arquivo do código fonte (.gleam) é obrigatório.';
			isError = true;
			isSubmitting = false;
			return;
		}

		const formData = new FormData();
		formData.append('title', title);
		formData.append('description', description);
		formData.append('source_code', sourceCodeFiles[0]);

		if (thumbnailFiles && thumbnailFiles.length > 0) {
			formData.append('thumbnail', thumbnailFiles[0]);
		}

		try {
			const response = await fetch('/api/games', {
				method: 'POST',
				body: formData
			});

			const result = await response.json();

			if (response.ok) {
				statusMessage = `Sucesso! Jogo #${result.id} submetido e aguardando aprovação.`;
				title = '';
				description = '';
				sourceCodeFiles = null as any;
				thumbnailFiles = null as any;
			} else {
				isError = true;
				statusMessage = `Erro do servidor: ${result}`;
			}
		} catch (error) {
			isError = true;
			statusMessage = 'Falha ao conectar com o servidor.';
			console.error(error);
		} finally {
			isSubmitting = false;
		}
	}
</script>

<main class="max-w-2xl mx-auto p-6 mt-10 bg-white rounded-lg shadow-md">
	<h1 class="text-3xl font-bold mb-6 text-gray-800">Publicar Novo Jogo</h1>

	<form on:submit|preventDefault={handleSubmit} class="space-y-6">
		
		<div>
			<label for="title" class="block text-sm font-medium text-gray-700">Título do Jogo</label>
			<input 
				type="text" 
				id="title" 
				bind:value={title} 
				required 
				class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm p-2 border"
				placeholder="Ex: Super Gleam Bros"
			/>
		</div>

		<div>
			<label for="description" class="block text-sm font-medium text-gray-700">Descrição</label>
			<textarea 
				id="description" 
				bind:value={description} 
				required 
				rows="3"
				class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm p-2 border"
				placeholder="Como jogar, qual a história..."
			></textarea>
		</div>

		<div>
			<label for="sourceCode" class="block text-sm font-medium text-gray-700">Código Fonte (.gleam)</label>
			<input 
				type="file" 
				id="sourceCode" 
				accept=".gleam"
				bind:files={sourceCodeFiles} 
				required 
				class="mt-1 block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-semibold file:bg-indigo-50 file:text-indigo-700 hover:file:bg-indigo-100"
			/>
		</div>

		<div>
			<label for="thumbnail" class="block text-sm font-medium text-gray-700">Capa do Jogo / Thumbnail (Opcional)</label>
			<input 
				type="file" 
				id="thumbnail" 
				accept="image/png, image/jpeg"
				bind:files={thumbnailFiles} 
				class="mt-1 block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-semibold file:bg-gray-50 file:text-gray-700 hover:file:bg-gray-100"
			/>
		</div>

		<button 
			type="submit" 
			disabled={isSubmitting}
			class="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50"
		>
			{isSubmitting ? 'Enviando...' : 'Submeter Jogo'}
		</button>
	</form>

	{#if statusMessage}
		<div class={`mt-4 p-4 rounded-md ${isError ? 'bg-red-50 text-red-700 border border-red-200' : 'bg-green-50 text-green-700 border border-green-200'}`}>
			{statusMessage}
		</div>
	{/if}
</main>
