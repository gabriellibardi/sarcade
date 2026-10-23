<script lang="ts">
	type Tone = 'info' | 'success' | 'warning' | 'error';

	let {
		message,
		title,
		tone = 'info',
		onRetry,
		dismissAfter,
		onDismiss
	}: {
		message: string;
		title?: string;
		tone?: Tone;
		onRetry?: () => void;
		dismissAfter?: number;
		onDismiss?: () => void;
	} = $props();

	let visible = $state(true);

	$effect(() => {
		if (!message) {
			visible = false;
			return;
		}

		visible = true;

		if (!dismissAfter) return;

		const timeout = window.setTimeout(() => {
			visible = false;
			onDismiss?.();
		}, dismissAfter);

		return () => window.clearTimeout(timeout);
	});

	const toneClasses: Record<Tone, string> = {
		info: 'border-indigo-300/40 bg-[#11182a]/90 text-indigo-900',
		success: 'border-emerald-600/50 bg-[#10241c]/90 text-emerald-100',
		warning: 'border-amber-500/50 bg-[#261e11]/90 text-amber-100',
		error: 'border-red-500/50 bg-[#291519]/90 text-red-100'
	};
</script>

{#if visible}
	<div
		class={`rounded-xl border p-4 shadow-xl backdrop-blur-md ${toneClasses[tone]}`}
		role={tone === 'error' ? 'alert' : 'status'}
	>
		{#if title}
			<p class="font-bold">{title}</p>
		{/if}
		<p class={title ? 'mt-1 text-sm leading-6' : 'text-sm leading-6'}>{message}</p>
		{#if onRetry}
			<button type="button" onclick={onRetry} class="mt-3 text-sm font-bold underline"
				>Tentar novamente</button
			>
		{/if}
	</div>
{/if}
