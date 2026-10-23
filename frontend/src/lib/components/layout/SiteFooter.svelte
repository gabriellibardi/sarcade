<script lang="ts">
	import { resolve } from '$app/paths';
	import BrandWordmark from '$lib/components/brand/BrandWordmark.svelte';
</script>

<footer class="relative bg-transparent">
	<div class="mx-auto max-w-5xl px-5 pt-4 pb-8 sm:px-6">
		<div
			class="h-px bg-gradient-to-r from-transparent via-arcade-red/70 to-transparent"
			aria-hidden="true"
		></div>

		<div class="flex flex-col gap-8 py-9 sm:flex-row sm:items-center sm:justify-between">
			<a href={resolve('/')} aria-label="Voltar ao início">
				<BrandWordmark size="footer" />
			</a>

			<p class="text-xs tracking-[0.16em] text-slate-500 uppercase">Crie / compartilhe / jogue</p>

			<nav class="flex items-center gap-2 text-sm text-slate-600" aria-label="Rodapé">
				<a
					href={resolve('/catalog')}
					class="rounded-full px-3 py-2 transition hover:bg-white/5 hover:text-white">Jogos</a
				>
				<a
					href={resolve('/submit')}
					class="rounded-full px-3 py-2 transition hover:bg-white/5 hover:text-white">Publicar</a
				>
			</nav>
		</div>

		<div
			class="flex flex-col gap-2 border-t border-white/8 pt-5 text-[11px] text-slate-500 sm:flex-row sm:items-center sm:justify-between"
		>
			<p>© 2026 Sarcade</p>
			<p>Universidade Estadual de Maringá</p>
		</div>
	</div>
</footer>
