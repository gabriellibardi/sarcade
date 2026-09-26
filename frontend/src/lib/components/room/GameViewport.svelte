<script lang="ts">
	let { frameUrl }: { frameUrl: string } = $props();
</script>

<section class="surface-card overflow-hidden">
	<div class="border-b-2 border-slate-200 px-6 py-4">
		<p class="text-xs font-bold tracking-widest text-slate-400 uppercase">Execução</p>
		<h2 class="mt-1 text-xl font-black text-slate-950">Tela do jogo</h2>
	</div>
	<div class="bg-[#0d0d0d] p-3 sm:p-5">
		{#if frameUrl}
			<img
				src={frameUrl}
				alt="Tela atual da partida"
				class="block h-auto min-h-64 w-full border-2 border-arcade-line bg-white object-contain"
			/>
		{:else}
			<div
				class="flex min-h-72 items-center justify-center border-2 border-dashed border-slate-700 text-center font-mono text-xs text-slate-400 uppercase"
			>
				O runtime ainda não enviou um quadro.
			</div>
		{/if}
	</div>
</section>
