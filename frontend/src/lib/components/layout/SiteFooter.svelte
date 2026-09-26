<script lang="ts">
	import { resolve } from '$app/paths';
	import BrandMark from '$lib/components/brand/BrandMark.svelte';
</script>

<footer class="border-t-2 border-arcade-line bg-[#101010]">
	<div
		class="mx-auto flex max-w-6xl flex-col gap-6 px-5 py-8 sm:flex-row sm:items-end sm:justify-between sm:px-6"
	>
		<a href={resolve('/')} class="flex items-center gap-4" aria-label="Voltar ao início">
			<BrandMark size="small" />
			<span>
				<strong class="display-title block text-xl text-slate-950">Sarcade</strong>
				<span class="block text-xs text-slate-500">Jogos feitos por estudantes.</span>
			</span>
		</a>
		<div
			class="font-mono text-[10px] leading-5 tracking-wider text-slate-500 uppercase sm:text-right"
		>
			<p>Linguagens educacionais + multiplayer</p>
			<p>UEM · Trabalho de Conclusão de Curso</p>
		</div>
	</div>
</footer>
