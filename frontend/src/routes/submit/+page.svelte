<script lang="ts">
	import { onMount } from 'svelte';

	type RuntimeBuild = {
		id: string;
		runtime: string;
		display_name: string;
		display_version: string;
		source_extension: string;
		status: string;
		is_default: boolean;
	};

	let title = '';
	let description = '';
	let minPlayers = 1;
	let maxPlayers = 1;

	let runtimeBuilds: RuntimeBuild[] = [];
	let runtimeBuildId = '';
	let isLoadingRuntimes = true;

	let sourceCodeFiles: FileList | null = null;
	let thumbnailFiles: FileList | null = null;

	let isSubmitting = false;
	let statusMessage = '';
	let isError = false;

	$: selectedRuntime = runtimeBuilds.find((build) => build.id === runtimeBuildId) ?? null;

	async function loadRuntimes() {
		isLoadingRuntimes = true;

		try {
			const response = await fetch('/api/runtimes');
			const result = await response.json().catch(() => null);

			if (!response.ok || !Array.isArray(result)) {
				isError = true;
				statusMessage = result?.message ?? 'Erro ao carregar linguagens disponíveis.';
				return;
			}

			runtimeBuilds = result;
			runtimeBuildId =
				runtimeBuilds.find((build) => build.is_default)?.id ?? runtimeBuilds[0]?.id ?? '';
		} catch (error) {
			isError = true;
			statusMessage = 'Falha ao carregar linguagens disponíveis.';
			console.error(error);
		} finally {
			isLoadingRuntimes = false;
		}
	}

	async function handleSubmit() {
		isSubmitting = true;
		statusMessage = '';
		isError = false;

		if (!selectedRuntime) {
			statusMessage = 'Selecione uma linguagem válida.';
			isError = true;
			isSubmitting = false;
			return;
		}

		if (!sourceCodeFiles || sourceCodeFiles.length === 0) {
			statusMessage = `O arquivo ${selectedRuntime.source_extension} é obrigatório.`;
			isError = true;
			isSubmitting = false;
			return;
		}

		if (
			!sourceCodeFiles[0].name
				.toLowerCase()
				.endsWith(selectedRuntime.source_extension.toLowerCase())
		) {
			statusMessage = `Escolha um arquivo ${selectedRuntime.source_extension}.`;
			isError = true;
			isSubmitting = false;
			return;
		}

		if (maxPlayers < minPlayers) {
			statusMessage = 'O máximo de jogadores deve ser maior ou igual ao mínimo.';
			isError = true;
			isSubmitting = false;
			return;
		}

		const formData = new FormData();
		formData.append('title', title);
		formData.append('description', description);
		formData.append('runtime_build_id', runtimeBuildId);
		formData.append('min_players', String(minPlayers));
		formData.append('max_players', String(maxPlayers));
		formData.append('source_code', sourceCodeFiles[0]);

		if (thumbnailFiles && thumbnailFiles.length > 0) {
			formData.append('thumbnail', thumbnailFiles[0]);
		}

		try {
			const response = await fetch('/api/games', {
				method: 'POST',
				body: formData
			});

			const result = await response.json().catch(() => null);

			if (!response.ok) {
				isError = true;
				statusMessage =
					typeof result === 'string' ? result : (result?.message ?? 'Erro ao submeter jogo.');
				return;
			}

			statusMessage =
				`Sucesso! Jogo #${result.id}, revisão #${result.revision_id}, ` +
				'submetido e aguardando aprovação.';

			title = '';
			description = '';
			minPlayers = 1;
			maxPlayers = 1;
			sourceCodeFiles = null;
			thumbnailFiles = null;
		} catch (error) {
			isError = true;
			statusMessage = 'Falha ao conectar com o servidor.';
			console.error(error);
		} finally {
			isSubmitting = false;
		}
	}

	onMount(loadRuntimes);
</script>

<main class="mx-auto max-w-2xl p-6">
	<h1 class="mb-2 text-2xl font-bold">Publicar jogo</h1>
	<p class="mb-6 text-sm text-gray-600">
		O build selecionado ficará associado a esta revisão para preservar sua compatibilidade.
	</p>

	<form on:submit|preventDefault={handleSubmit} class="space-y-4">
		<div>
			<label for="title" class="block font-medium">Título</label>
			<input
				id="title"
				type="text"
				bind:value={title}
				required
				maxlength="100"
				class="block w-full rounded border p-2"
			/>
		</div>

		<div>
			<label for="description" class="block font-medium">Descrição</label>
			<textarea
				id="description"
				bind:value={description}
				required
				maxlength="2000"
				rows="4"
				class="block w-full rounded border p-2"
			></textarea>
		</div>

		<div>
			<label for="runtimeBuild" class="block font-medium">Linguagem e versão</label>
			<select
				id="runtimeBuild"
				bind:value={runtimeBuildId}
				disabled={isLoadingRuntimes}
				required
				class="block w-full rounded border p-2"
			>
				{#if isLoadingRuntimes}
					<option value="">Carregando...</option>
				{:else}
					{#each runtimeBuilds as build}
						<option value={build.id}>
							{build.display_name} — {build.display_version}
						</option>
					{/each}
				{/if}
			</select>
		</div>

		<div class="grid grid-cols-2 gap-4">
			<div>
				<label for="minPlayers" class="block font-medium">Jogadores mínimos</label>
				<input
					id="minPlayers"
					type="number"
					min="1"
					bind:value={minPlayers}
					required
					class="block w-full rounded border p-2"
				/>
			</div>

			<div>
				<label for="maxPlayers" class="block font-medium">Jogadores máximos</label>
				<input
					id="maxPlayers"
					type="number"
					min="1"
					bind:value={maxPlayers}
					required
					class="block w-full rounded border p-2"
				/>
			</div>
		</div>

		<div>
			<p class="mb-1 font-medium">
				Código-fonte
				{#if selectedRuntime}
					({selectedRuntime.source_extension})
				{/if}
			</p>

			<input
				id="sourceCode"
				type="file"
				accept={selectedRuntime?.source_extension ?? ''}
				bind:files={sourceCodeFiles}
				required
				class="hidden"
			/>

			<label
				for="sourceCode"
				class="inline-block cursor-pointer rounded border bg-gray-100 px-4 py-2 hover:bg-gray-200"
			>
				Escolher arquivo
				{selectedRuntime?.source_extension ?? ''}
			</label>

			{#if sourceCodeFiles?.length}
				<p class="mt-1 text-sm">Arquivo selecionado: {sourceCodeFiles[0].name}</p>
			{/if}
		</div>

		<div>
			<p class="mb-1 font-medium">Thumbnail (opcional)</p>

			<input
				id="thumbnail"
				type="file"
				accept="image/png,image/jpeg,image/webp"
				bind:files={thumbnailFiles}
				class="hidden"
			/>

			<label
				for="thumbnail"
				class="inline-block cursor-pointer rounded border bg-gray-100 px-4 py-2 hover:bg-gray-200"
			>
				Escolher imagem
			</label>

			{#if thumbnailFiles?.length}
				<p class="mt-1 text-sm">Imagem selecionada: {thumbnailFiles[0].name}</p>
			{/if}
		</div>

		<button
			type="submit"
			disabled={isSubmitting || isLoadingRuntimes || !selectedRuntime}
			class="rounded bg-black px-4 py-2 text-white disabled:opacity-50"
		>
			{isSubmitting ? 'Enviando...' : 'Enviar'}
		</button>
	</form>

	{#if statusMessage}
		<div
			class={`mt-4 rounded border p-3 ${
				isError ? 'border-red-300 text-red-700' : 'border-green-300 text-green-700'
			}`}
		>
			{statusMessage}
		</div>
	{/if}
</main>
