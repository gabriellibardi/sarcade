<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
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

			<nav
				aria-label="Navegação principal"
				class="flex flex-wrap items-center gap-1 text-sm font-medium"
			>
				<a href="/catalog" class="nav-link">Catálogo</a>
				<a href="/rooms/new" class="nav-link">Criar sala</a>
				<a href="/submit" class="nav-link">Publicar jogo</a>
				<a href="/pending" class="nav-link">Revisões</a>
			</nav>
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
