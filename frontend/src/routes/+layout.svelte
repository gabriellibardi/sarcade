<script lang="ts">
	import { onMount } from 'svelte';
	import { authState, loadAuth, logout } from '$lib/auth';
	import favicon from '$lib/assets/favicon.svg';
	import './layout.css';

	let { children } = $props();
	let isLoggingOut = $state(false);

	async function handleLogout() {
		isLoggingOut = true;

		try {
			await logout();
		} catch (error) {
			console.error(error);
		} finally {
			isLoggingOut = false;
		}
	}

	onMount(loadAuth);
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<meta
		name="description"
		content="Sarcade: jogos educacionais criados com linguagens para estudantes."
	/>
</svelte:head>

<div class="flex min-h-screen flex-col">
	<header class="border-b border-slate-200 bg-white/95 backdrop-blur">
		<div
			class="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4 px-5 py-4 sm:px-6"
		>
			<a href="/" class="flex items-center gap-3" aria-label="Página inicial do Sarcade">
				<span
					class="flex h-10 w-10 items-center justify-center rounded-xl bg-indigo-600 text-xl font-black text-white shadow-sm"
				>
					S
				</span>
				<span>
					<strong class="block text-lg leading-none tracking-tight text-slate-950">Sarcade</strong>
					<span class="text-xs text-slate-500">Jogos feitos por estudantes</span>
				</span>
			</a>

			<div class="flex flex-wrap items-center justify-end gap-2">
				<nav
					aria-label="Navegação principal"
					class="flex flex-wrap items-center gap-1 text-sm font-medium"
				>
					<a href="/catalog" class="nav-link">Catálogo</a>
					<a href="/rooms/new" class="nav-link">Criar sala</a>
					<a href="/submit" class="nav-link">Publicar jogo</a>
					{#if $authState.user?.is_admin}
						<a href="/pending" class="nav-link">Revisões</a>
					{/if}
				</nav>

				<div class="ml-1 border-l border-slate-200 pl-3">
					{#if $authState.isLoading}
						<span
							class="block h-9 w-24 animate-pulse rounded-xl bg-slate-100"
							aria-label="Verificando login"
						></span>
					{:else if $authState.user}
						<div class="flex items-center gap-2">
							{#if $authState.user.avatar}
								<img
									src={$authState.user.avatar}
									alt=""
									referrerpolicy="no-referrer"
									class="h-9 w-9 rounded-full border border-slate-200 object-cover"
								/>
							{:else}
								<span
									class="flex h-9 w-9 items-center justify-center rounded-full bg-indigo-100 font-bold text-indigo-700"
								>
									{$authState.user.name.slice(0, 1).toUpperCase()}
								</span>
							{/if}
							<div class="hidden max-w-32 sm:block">
								<div class="flex items-center gap-1.5">
									<p class="truncate text-sm font-bold text-slate-800">{$authState.user.name}</p>
									{#if $authState.user.is_admin}
										<span class="rounded bg-indigo-100 px-1.5 py-0.5 text-[10px] font-bold text-indigo-700">
											Admin
										</span>
									{/if}
								</div>
								<p class="truncate text-xs text-slate-500">{$authState.user.email}</p>
							</div>
							<button
								type="button"
								onclick={handleLogout}
								disabled={isLoggingOut}
								class="rounded-lg px-2.5 py-2 text-sm font-semibold text-slate-500 hover:bg-slate-100 hover:text-slate-900 disabled:opacity-50"
							>
								{isLoggingOut ? 'Saindo...' : 'Sair'}
							</button>
						</div>
					{:else if $authState.googleOAuthEnabled}
						<a
							href="/api/auth/start?return_to=/"
							class="inline-flex items-center gap-2 rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm font-bold text-slate-700 shadow-sm transition hover:border-indigo-300 hover:bg-indigo-50"
						>
							<svg viewBox="0 0 24 24" aria-hidden="true" class="h-4 w-4">
								<path
									fill="#4285F4"
									d="M21.6 12.2c0-.7-.1-1.4-.2-2H12v3.9h5.4a4.6 4.6 0 0 1-2 3v2.5h3.3c1.9-1.8 2.9-4.4 2.9-7.4Z"
								/>
								<path
									fill="#34A853"
									d="M12 22c2.7 0 5-.9 6.7-2.4l-3.3-2.5c-.9.6-2.1 1-3.4 1a5.9 5.9 0 0 1-5.5-4.1H3.1v2.6A10 10 0 0 0 12 22Z"
								/>
								<path
									fill="#FBBC05"
									d="M6.5 14a6 6 0 0 1 0-3.9V7.4H3.1A10 10 0 0 0 3.1 16.6L6.5 14Z"
								/>
								<path
									fill="#EA4335"
									d="M12 5.9c1.5 0 2.8.5 3.9 1.5l2.9-2.9A9.8 9.8 0 0 0 3.1 7.4l3.4 2.7A5.9 5.9 0 0 1 12 5.9Z"
								/>
							</svg>
							Entrar
						</a>
					{:else}
						<span
							class="rounded-lg bg-slate-100 px-3 py-2 text-xs font-semibold text-slate-500"
							title="Defina as credenciais Google no backend"
						>
							Login não configurado
						</span>
					{/if}
				</div>
			</div>
		</div>
	</header>

	<div class="flex-1">{@render children()}</div>

	<footer class="border-t border-slate-200 bg-white">
		<div
			class="mx-auto flex max-w-6xl flex-col gap-1 px-5 py-6 text-sm text-slate-500 sm:flex-row sm:items-center sm:justify-between sm:px-6"
		>
			<p>Sarcade — arcade para Student Languages.</p>
			<p>Sgleam e Spython em um só lugar.</p>
		</div>
	</footer>
</div>
