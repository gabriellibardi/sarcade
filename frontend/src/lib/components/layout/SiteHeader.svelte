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

<header
	class="sticky top-0 z-30 border-b-2 border-arcade-line bg-arcade-bg/95 shadow-[0_4px_0_#511243] backdrop-blur-xl"
>
	<div class="mx-auto flex max-w-6xl items-center justify-between gap-6 px-5 py-3 sm:px-6">
		<a href={resolve('/')} class="flex items-center gap-3" aria-label="Página inicial do Sarcade">
			<BrandMark size="medium" />
		</a>

		<div class="hidden flex-1 items-center justify-end gap-2 lg:flex">
			<nav aria-label="Navegação principal" class="flex items-center gap-1">
				<a href={resolve('/catalog')} class="nav-link">Jogos</a>
				<a href={resolve('/submit')} class="nav-link">Publicar</a>
				{#if $authState.user}
					<a href={resolve('/my-games')} class="nav-link">Meus jogos</a>
				{/if}
				{#if $authState.user?.is_admin}
					<a href={resolve('/pending')} class="nav-link">Revisões</a>
				{/if}
			</nav>

			<div class="ml-1 border-l-2 border-arcade-line pl-3">
				{#if $authState.isLoading}
					<span class="block h-9 w-24 animate-pulse bg-slate-100" aria-label="Verificando login"
					></span>
				{:else if $authState.user}
					<div class="flex items-center gap-2">
						<a
							href={resolve('/users/[id]', { id: String($authState.user.id) })}
							class="flex items-center gap-2 border border-transparent p-1 transition hover:border-arcade-line hover:bg-slate-100"
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
						class="secondary-button px-3 py-2 text-xs"
					>
						Entrar
					</a>
				{:else}
					<span
						class="border border-arcade-line bg-slate-100 px-3 py-2 font-mono text-[10px] font-semibold text-slate-500 uppercase"
						title="Defina as credenciais Google no backend"
					>
						Login offline
					</span>
				{/if}
			</div>
		</div>

		<button
			type="button"
			class="ghost-button gap-2 border-arcade-line px-3 py-2 text-xs lg:hidden"
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
			class="absolute inset-x-0 top-full border-b-2 border-arcade-line bg-arcade-bg p-5 shadow-[0_4px_0_#511243] lg:hidden"
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

			<div class="mt-4 border-t-2 border-arcade-line pt-4">
				{#if $authState.isLoading}
					<span class="block h-10 w-full animate-pulse bg-slate-100" aria-label="Verificando login"
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
						class="secondary-button w-full px-3 py-2 text-xs"
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
