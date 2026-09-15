<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { ApiError, apiRequest, errorMessage } from '$lib/api';
	import GameCard from '$lib/components/GameCard.svelte';
	import Alert from '$lib/components/ui/Alert.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import LoadingCards from '$lib/components/ui/LoadingCards.svelte';
	import UserAvatar from '$lib/components/ui/UserAvatar.svelte';
	import type { PublicUserProfile } from '$lib/types/user';
	import { formatDate } from '$lib/utils/format';

	let profile: PublicUserProfile | null = null;
	let isLoading = true;
	let notFound = false;
	let error = '';

	const userId = Number(page.params.id);

	async function loadProfile() {
		isLoading = true;
		notFound = false;
		error = '';

		if (!Number.isSafeInteger(userId) || userId <= 0) {
			notFound = true;
			isLoading = false;
			return;
		}

		try {
			profile = await apiRequest<PublicUserProfile>(`/api/users/${userId}`);
		} catch (loadError) {
			if (loadError instanceof ApiError && loadError.status === 404) {
				notFound = true;
				return;
			}

			error = errorMessage(loadError, 'Não foi possível carregar este perfil.');
			console.error(loadError);
		} finally {
			isLoading = false;
		}
	}

	onMount(loadProfile);
</script>

<svelte:head>
	<title>{profile ? `${profile.name} — Sarcade` : 'Perfil — Sarcade'}</title>
	<meta name="description" content="Veja os jogos publicados por um autor no Sarcade." />
</svelte:head>

<main class="mx-auto max-w-6xl px-5 py-12 sm:px-6 lg:py-16">
	<a href={resolve('/catalog')} class="text-sm font-bold text-indigo-700 hover:underline"
		>← Voltar ao catálogo</a
	>

	{#if isLoading}
		<div class="mt-7"><LoadingCards count={1} /></div>
	{:else if notFound}
		<div class="mt-7">
			<EmptyState
				title="Usuário não encontrado"
				description="Não encontramos um perfil com esse identificador."
			>
				<a href={resolve('/catalog')} class="primary-button">Explorar o catálogo</a>
			</EmptyState>
		</div>
	{:else if error}
		<div class="mt-7"><Alert tone="error" message={error} onRetry={loadProfile} /></div>
	{:else if profile}
		<header
			class="mt-7 flex flex-col gap-5 rounded-3xl border border-slate-200 bg-white p-7 shadow-sm sm:flex-row sm:items-center"
		>
			<UserAvatar name={profile.name} avatar={profile.avatar} size="large" />

			<div>
				<p class="text-sm font-bold tracking-widest text-indigo-600 uppercase">Autor no Sarcade</p>
				<h1 class="mt-1 text-4xl font-black tracking-tight text-slate-950">{profile.name}</h1>
				<p class="mt-2 text-slate-600">
					Membro desde {formatDate(profile.created_at, { month: 'long', year: 'numeric' }) ??
						'data não informada'} · {profile.published_games.length === 1
						? '1 jogo publicado'
						: `${profile.published_games.length} jogos publicados`}
				</p>
			</div>
		</header>

		<section class="mt-12">
			<h2 class="text-3xl font-black tracking-tight text-slate-950">Jogos publicados</h2>

			{#if profile.published_games.length === 0}
				<div class="mt-6">
					<EmptyState
						title="Nenhum jogo publicado"
						description="Este autor ainda não possui jogos aprovados no catálogo."
					/>
				</div>
			{:else}
				<div class="mt-6 grid gap-5 md:grid-cols-2 lg:grid-cols-3">
					{#each profile.published_games as game (game.id)}
						<GameCard {game} />
					{/each}
				</div>
			{/if}
		</section>
	{/if}
</main>
