<script lang="ts">
	import { resolve } from '$app/paths';
	import BrandMark from '$lib/components/brand/BrandMark.svelte';
</script>

<footer class="border-t border-arcade-line bg-[#101010]">
	<div
		class="mx-auto flex max-w-6xl flex-col gap-6 px-5 py-8 sm:flex-row sm:items-center sm:justify-between sm:px-6"
	>
		<a href={resolve('/')} class="flex items-center gap-3" aria-label="Voltar ao início">
			<BrandMark size="small" />
			<span>
				<strong class="display-title block text-xl text-slate-950">Sarcade</strong>
				<span class="block text-xs text-slate-500">Crie, compartilhe e jogue.</span>
			</span>
		</a>
		<nav
			class="flex items-center gap-5 font-mono text-xs font-bold text-slate-500"
			aria-label="Rodapé"
		>
			<a href={resolve('/catalog')} class="transition hover:text-slate-900">Jogos</a>
			<a href={resolve('/submit')} class="transition hover:text-slate-900">Publicar</a>
		</nav>
	</div>
</footer>
