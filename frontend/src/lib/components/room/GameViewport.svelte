<script lang="ts">
	let {
		frameUrl,
		sinteract,
		canvas = $bindable(null)
	}: {
		frameUrl: string;
		sinteract: boolean;
		canvas?: HTMLCanvasElement | null;
	} = $props();
</script>

<section class="surface-card overflow-hidden">
	<div class="hidden border-b-2 border-slate-200 px-6 py-4 sm:block">
		<p class="text-xs font-bold tracking-widest text-slate-400 uppercase">Execução</p>
		<h2 class="mt-1 text-xl font-black text-slate-950">Tela do jogo</h2>
	</div>
	<div class="bg-[#0d0d0d] p-1 sm:p-5">
		{#if sinteract}
			<canvas
				bind:this={canvas}
				tabindex="0"
				aria-label="Tela interativa da partida"
				class="block aspect-video h-auto min-h-64 w-full touch-none border-2 border-arcade-line bg-white object-contain outline-none focus:border-indigo-300"
			></canvas>
		{:else if frameUrl}
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
