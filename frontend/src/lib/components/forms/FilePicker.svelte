<script lang="ts">
	let {
		id,
		label,
		accept,
		required = false,
		files = $bindable<FileList | null>(null),
		buttonLabel = 'Escolher arquivo'
	}: {
		id: string;
		label: string;
		accept: string;
		required?: boolean;
		files?: FileList | null;
		buttonLabel?: string;
	} = $props();
</script>

<div>
	<p class="form-label">{label}</p>
	<input {id} type="file" {accept} bind:files {required} class="sr-only" />
	<label for={id} class="secondary-button cursor-pointer px-4 py-2 text-sm">{buttonLabel}</label>
	{#if files?.length}
		<p class="mt-3 truncate font-mono text-xs text-slate-600">
			Selecionado: <strong>{files[0].name}</strong>
		</p>
	{/if}
</div>
