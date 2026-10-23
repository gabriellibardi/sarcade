<script lang="ts">
	import { resolve } from '$app/paths';
	import { authState, logout } from '$lib/auth';
	import BrandMark from '$lib/components/brand/BrandMark.svelte';
	import UserAvatar from '$lib/components/ui/UserAvatar.svelte';

	let isLoggingOut = $state(false);
	let menuOpen = $state(false);

	async function handleLogout() {
		isLoggingOut = true;

		try {
			await logout();
		} catch (error) {
			console.error(error);
		} finally {
			isLoggingOut = false;
			menuOpen = false;
		}
	}
</script>

<header class="relative z-30 bg-transparent">
	<div
		class="mx-auto grid max-w-5xl grid-cols-[1fr_auto] items-center gap-6 px-5 py-3.5 sm:px-6 lg:grid-cols-[1fr_auto_1fr]"
	>
		<a href={resolve('/')} class="flex items-center gap-3" aria-label="Página inicial do Sarcade">
			<BrandMark size="medium" />
		</a>

		<div class="hidden lg:contents">
			<nav aria-label="Navegação principal" class="flex items-center gap-2 justify-self-center">
				<a href={resolve('/catalog')} class="nav-link">Jogos</a>
				<a href={resolve('/submit')} class="nav-link">Publicar</a>
				{#if $authState.user}
					<a href={resolve('/my-games')} class="nav-link">Meus jogos</a>
				{/if}
				{#if $authState.user?.is_admin}
					<a href={resolve('/pending')} class="nav-link">Revisões</a>
				{/if}
			</nav>

			<div class="justify-self-end">
				{#if $authState.isLoading}
					<span
						class="block h-9 w-24 animate-pulse rounded-full bg-slate-100"
						aria-label="Verificando login"
					></span>
				{:else if $authState.user}
					<div class="flex items-center gap-2">
						<a
							href={resolve('/users/[id]', { id: String($authState.user.id) })}
							class="flex items-center gap-2 rounded-full border border-transparent p-1 transition hover:border-white/10 hover:bg-white/5"
						>
							<UserAvatar
								name={$authState.user.name}
								avatar={$authState.user.avatar}
								size="small"
							/>
							<div class="hidden max-w-32 sm:block">
								<div class="flex items-center gap-1.5">
									<p class="truncate text-sm font-bold text-slate-800">{$authState.user.name}</p>
									{#if $authState.user.is_admin}
										<span
											class="status-chip bg-indigo-100 px-1.5 py-0.5 text-[9px] font-bold text-indigo-300"
											>Admin</span
										>
									{/if}
								</div>
								<p class="truncate font-mono text-[10px] text-slate-500">{$authState.user.email}</p>
							</div>
						</a>
						<button
							type="button"
							onclick={handleLogout}
							disabled={isLoggingOut}
							class="ghost-button px-2.5 py-2 text-xs disabled:opacity-50"
						>
							{isLoggingOut ? 'Saindo...' : 'Sair'}
						</button>
					</div>
				{:else if $authState.googleOAuthEnabled}
					<a
						href="/api/auth/start?return_to=/"
						rel="external"
						class="primary-button min-w-24 px-5 py-2 text-xs"
					>
						Entrar
					</a>
				{:else}
					<span
						class="rounded-full border border-white/10 bg-slate-100/80 px-3 py-2 text-[10px] font-medium text-slate-500"
						title="Defina as credenciais Google no backend"
					>
						Login offline
					</span>
				{/if}
			</div>
		</div>

		<button
			type="button"
			class="ghost-button gap-2 border-white/12 px-3 py-2 text-xs lg:hidden"
			aria-controls="mobile-navigation"
			aria-expanded={menuOpen}
			onclick={() => (menuOpen = !menuOpen)}
		>
			<svg viewBox="0 0 20 20" class="h-4 w-4" aria-hidden="true">
				<path
					d="M3 5h14M3 10h14M3 15h14"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="square"
				/>
			</svg>
			Menu
		</button>
	</div>

	{#if menuOpen}
		<div
			id="mobile-navigation"
			class="absolute inset-x-0 top-full border-b border-white/10 bg-[#080b0e]/96 p-5 shadow-2xl backdrop-blur-xl lg:hidden"
		>
			<nav aria-label="Navegação principal no celular" class="grid gap-1">
				<a href={resolve('/catalog')} class="nav-link" onclick={() => (menuOpen = false)}>Jogos</a>
				<a href={resolve('/submit')} class="nav-link" onclick={() => (menuOpen = false)}>Publicar</a
				>
				{#if $authState.user}
					<a href={resolve('/my-games')} class="nav-link" onclick={() => (menuOpen = false)}
						>Meus jogos</a
					>
				{/if}
				{#if $authState.user?.is_admin}
					<a href={resolve('/pending')} class="nav-link" onclick={() => (menuOpen = false)}
						>Revisões</a
					>
				{/if}
			</nav>

			<div class="mt-4 border-t border-white/10 pt-4">
				{#if $authState.isLoading}
					<span
						class="block h-10 w-full animate-pulse rounded-full bg-slate-100"
						aria-label="Verificando login"
					></span>
				{:else if $authState.user}
					<div class="flex items-center justify-between gap-3">
						<a
							href={resolve('/users/[id]', { id: String($authState.user.id) })}
							class="flex min-w-0 items-center gap-3"
							onclick={() => (menuOpen = false)}
						>
							<UserAvatar
								name={$authState.user.name}
								avatar={$authState.user.avatar}
								size="small"
							/>
							<span class="truncate text-sm font-bold text-slate-800">{$authState.user.name}</span>
						</a>
						<button
							type="button"
							onclick={handleLogout}
							disabled={isLoggingOut}
							class="ghost-button shrink-0 px-3 py-2 text-xs disabled:opacity-50"
						>
							{isLoggingOut ? 'Saindo...' : 'Sair'}
						</button>
					</div>
				{:else if $authState.googleOAuthEnabled}
					<a
						href="/api/auth/start?return_to=/"
						rel="external"
						class="primary-button w-full px-3 py-2 text-xs"
					>
						Entrar
					</a>
				{:else}
					<p class="font-mono text-xs text-slate-500">Login indisponível</p>
				{/if}
			</div>
		</div>
	{/if}
</header>
