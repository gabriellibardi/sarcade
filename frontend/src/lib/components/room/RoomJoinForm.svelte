<script lang="ts">
	let {
		nickname = $bindable(''),
		isConnecting,
		onJoin
	}: { nickname?: string; isConnecting: boolean; onJoin: () => void } = $props();
</script>

<section class="surface-card p-6 sm:p-8">
	<p class="eyebrow">Participar</p>
	<h2 class="mt-2 text-2xl font-black text-slate-950">Entrar na sala</h2>
	<p class="mt-2 text-sm leading-6 text-slate-600">
		Informe como os demais participantes verão você durante a partida.
	</p>

	<form
		onsubmit={(event) => {
			event.preventDefault();
			onJoin();
		}}
		class="mt-5 flex flex-col gap-3 sm:flex-row"
	>
		<label class="flex-1">
			<span class="sr-only">Nickname</span>
			<input
				type="text"
				bind:value={nickname}
				required
				maxlength="30"
				autocomplete="nickname"
				placeholder="Seu nickname"
				class="form-control"
			/>
		</label>
		<button type="submit" disabled={isConnecting} class="primary-button sm:min-w-32">
			{isConnecting ? 'Conectando...' : 'Entrar'}
		</button>
	</form>
</section>
