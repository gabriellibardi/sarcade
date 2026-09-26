<script lang="ts">
	import { onMount } from 'svelte';
	import { ApiError, apiRequest, errorMessage } from '$lib/api';
	import { authState, loadAuth } from '$lib/auth';
	import FilePicker from '$lib/components/forms/FilePicker.svelte';
	import Alert from '$lib/components/ui/Alert.svelte';
	import PageHeader from '$lib/components/ui/PageHeader.svelte';
	import UserAvatar from '$lib/components/ui/UserAvatar.svelte';
	import type { RuntimeBuild } from '$lib/types/runtime';

	type SubmissionResponse = { id: number; revision_id: number };

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
			runtimeBuilds = await apiRequest<RuntimeBuild[]>('/api/runtimes');
			runtimeBuildId =
				runtimeBuilds.find((build) => build.is_default)?.id ?? runtimeBuilds[0]?.id ?? '';
		} catch (loadError) {
			isError = true;
			statusMessage = errorMessage(
				loadError,
				'Não foi possível carregar as linguagens disponíveis.'
			);
			console.error(loadError);
		} finally {
			isLoadingRuntimes = false;
		}
	}

	function validationError(): string | null {
		if (!$authState.user) return 'Faça login com o Google para publicar.';
		if (!selectedRuntime) return 'Selecione uma linguagem válida.';
		if (!sourceCodeFiles?.length)
			return `O arquivo ${selectedRuntime.source_extension} é obrigatório.`;
		if (
			!sourceCodeFiles[0].name
				.toLowerCase()
				.endsWith(selectedRuntime.source_extension.toLowerCase())
		) {
			return `Escolha um arquivo ${selectedRuntime.source_extension}.`;
		}
		if (maxPlayers < minPlayers) return 'O máximo de jogadores deve ser maior ou igual ao mínimo.';
		return null;
	}

	async function handleSubmit() {
		statusMessage = '';
		isError = false;

		const invalidMessage = validationError();
		if (invalidMessage) {
			statusMessage = invalidMessage;
			isError = true;
			return;
		}

		const runtime = selectedRuntime;
		const sourceCode = sourceCodeFiles?.[0];
		if (!runtime || !sourceCode) return;

		isSubmitting = true;
		const formData = new FormData();
		formData.append('title', title.trim());
		formData.append('description', description.trim());
		formData.append('runtime_build_id', runtime.id);
		formData.append('min_players', String(minPlayers));
		formData.append('max_players', String(maxPlayers));
		formData.append('source_code', sourceCode);

		if (thumbnailFiles?.length) {
			formData.append('thumbnail', thumbnailFiles[0]);
		}

		try {
			const result = await apiRequest<SubmissionResponse>('/api/games', {
				method: 'POST',
				body: formData
			});

			statusMessage = `Jogo #${result.id}, revisão #${result.revision_id}, enviado e aguardando aprovação.`;
			title = '';
			description = '';
			minPlayers = 1;
			maxPlayers = 1;
			sourceCodeFiles = null;
			thumbnailFiles = null;
		} catch (submitError) {
			if (submitError instanceof ApiError && submitError.status === 401) {
				await loadAuth(true);
			}
			isError = true;
			statusMessage = errorMessage(submitError, 'Não foi possível enviar o jogo.');
			console.error(submitError);
		} finally {
			isSubmitting = false;
		}
	}

	onMount(() => {
		void loadAuth();
		void loadRuntimes();
	});
</script>

<svelte:head>
	<title>Publicar jogo — Sarcade</title>
	<meta name="description" content="Envie um jogo educacional para revisão no Sarcade." />
</svelte:head>

<main class="page-shell max-w-5xl">
	<PageHeader
		eyebrow="Compartilhe sua criação"
		title="Publicar jogo"
		description="Envie o código, a apresentação e os limites de participantes. A equipe revisará o projeto antes de publicá-lo."
	/>

	<div class="mt-9">
		{#if $authState.isLoading}
			<div class="surface-card h-40 animate-pulse" aria-label="Verificando sessão"></div>
		{:else if !$authState.user}
			<section class="arcade-panel-blue p-7">
				<p class="eyebrow">Autenticação necessária</p>
				<h2 class="display-title mt-3 text-2xl text-slate-950">Entre para publicar</h2>
				<p class="mt-2 max-w-2xl leading-7 text-slate-600">
					O login identifica o autor, protege a submissão e permite acompanhar a revisão.
				</p>
				{#if $authState.googleOAuthEnabled}
					<a
						href="/api/auth/start?return_to=/submit"
						rel="external"
						class="primary-button mt-5 gap-2"
					>
						<span class="font-black text-white" aria-hidden="true">G</span>
						Entrar com Google
					</a>
				{:else}
					<div class="mt-5">
						<Alert
							tone="warning"
							message="O administrador ainda precisa configurar as credenciais Google."
						/>
					</div>
				{/if}
			</section>
		{:else}
			<div class="grid items-start gap-7 lg:grid-cols-[1fr_18rem]">
				<form
					onsubmit={(event) => {
						event.preventDefault();
						void handleSubmit();
					}}
					class="surface-card space-y-6 p-6 sm:p-8"
				>
					<div>
						<label for="title" class="form-label">Título</label>
						<input
							id="title"
							type="text"
							bind:value={title}
							required
							maxlength="100"
							class="form-control"
							placeholder="Nome do jogo"
						/>
					</div>

					<div>
						<label for="description" class="form-label">Descrição</label>
						<textarea
							id="description"
							bind:value={description}
							required
							maxlength="2000"
							rows="5"
							class="form-control"
							placeholder="Explique a proposta e como jogar"
						></textarea>
						<p class="mt-1.5 text-right text-xs text-slate-400">{description.length}/2000</p>
					</div>

					<div>
						<label for="runtimeBuild" class="form-label">Linguagem e versão</label>
						<select
							id="runtimeBuild"
							bind:value={runtimeBuildId}
							disabled={isLoadingRuntimes}
							required
							class="form-control"
						>
							{#if isLoadingRuntimes}
								<option value="">Carregando...</option>
							{:else}
								{#each runtimeBuilds as build (build.id)}
									<option value={build.id}>{build.display_name} — {build.display_version}</option>
								{/each}
							{/if}
						</select>
					</div>

					<fieldset>
						<legend class="form-label">Participantes</legend>
						<div class="grid gap-4 sm:grid-cols-2">
							<label>
								<span class="mb-1.5 block text-sm text-slate-600">Mínimo</span>
								<input
									type="number"
									min="1"
									bind:value={minPlayers}
									required
									class="form-control"
								/>
							</label>
							<label>
								<span class="mb-1.5 block text-sm text-slate-600">Máximo</span>
								<input
									type="number"
									min="1"
									bind:value={maxPlayers}
									required
									class="form-control"
								/>
							</label>
						</div>
					</fieldset>

					<div class="grid gap-5 border-t-2 border-slate-200 pt-6 sm:grid-cols-2">
						<FilePicker
							id="sourceCode"
							label={`Código-fonte ${selectedRuntime?.source_extension ?? ''}`}
							accept={selectedRuntime?.source_extension ?? ''}
							bind:files={sourceCodeFiles}
							required
						/>
						<FilePicker
							id="thumbnail"
							label="Capa do jogo (opcional)"
							accept="image/png,image/jpeg,image/webp"
							bind:files={thumbnailFiles}
							buttonLabel="Escolher imagem"
						/>
					</div>

					<button
						type="submit"
						disabled={isSubmitting || isLoadingRuntimes || !selectedRuntime}
						class="primary-button w-full sm:w-auto"
					>
						{isSubmitting ? 'Enviando para revisão...' : 'Enviar para revisão'}
					</button>
				</form>

				<aside class="space-y-5 lg:sticky lg:top-24">
					<section class="surface-card p-5">
						<div class="flex items-center gap-3">
							<UserAvatar name={$authState.user.name} avatar={$authState.user.avatar} />
							<div class="min-w-0">
								<p class="text-xs font-bold text-slate-400 uppercase">Publicando como</p>
								<p class="truncate font-black text-slate-900">{$authState.user.name}</p>
							</div>
						</div>
					</section>

					<section class="arcade-panel-blue p-5 text-slate-300">
						<p class="text-xs font-bold tracking-widest text-indigo-300 uppercase">
							Compatibilidade
						</p>
						<h2 class="mt-2 font-black text-white">Build fixado por revisão</h2>
						<p class="mt-2 text-sm leading-6">
							A linguagem e sua versão ficam associadas à submissão para tornar a avaliação
							reproduzível.
						</p>
					</section>
				</aside>
			</div>
		{/if}

		{#if statusMessage}
			<div class="mt-5"><Alert tone={isError ? 'error' : 'success'} message={statusMessage} /></div>
		{/if}
	</div>
</main>
