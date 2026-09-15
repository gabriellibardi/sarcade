<script lang="ts">
	import { resolve } from '$app/paths';
	import { authState, logout } from '$lib/auth';
	import UserAvatar from '$lib/components/ui/UserAvatar.svelte';

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
</script>

<header class="sticky top-0 z-30 border-b border-slate-200/80 bg-white/90 backdrop-blur-xl">
	<div
		class="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4 px-5 py-3 sm:px-6"
	>
		<a href={resolve('/')} class="flex items-center gap-3" aria-label="Página inicial do Sarcade">
			<span
				class="flex h-10 w-10 items-center justify-center rounded-xl bg-indigo-600 text-xl font-black text-white shadow-sm"
				>S</span
			>
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
				<a href={resolve('/catalog')} class="nav-link">Catálogo</a>
				<a href={resolve('/submit')} class="nav-link">Publicar jogo</a>
				{#if $authState.user?.is_admin}
					<a href={resolve('/pending')} class="nav-link">Revisões</a>
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
						<a
							href={resolve('/users/[id]', { id: String($authState.user.id) })}
							class="flex items-center gap-2 rounded-xl p-1 transition hover:bg-slate-100"
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
											class="rounded bg-indigo-100 px-1.5 py-0.5 text-[10px] font-bold text-indigo-700"
											>Admin</span
										>
									{/if}
								</div>
								<p class="truncate text-xs text-slate-500">{$authState.user.email}</p>
							</div>
						</a>
						<button
							type="button"
							onclick={handleLogout}
							disabled={isLoggingOut}
							class="ghost-button px-2.5 py-2 text-sm disabled:opacity-50"
						>
							{isLoggingOut ? 'Saindo...' : 'Sair'}
						</button>
					</div>
				{:else if $authState.googleOAuthEnabled}
					<a
						href="/api/auth/start?return_to=/"
						rel="external"
						class="secondary-button gap-2 px-3 py-2 text-sm"
					>
						<span aria-hidden="true" class="font-black text-indigo-600">G</span>
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
