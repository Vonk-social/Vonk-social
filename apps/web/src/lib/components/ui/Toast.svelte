<script lang="ts">
	import { toasts } from '$lib/stores/toasts';
</script>

<div
	class="pointer-events-none fixed inset-x-0 bottom-6 z-50 flex flex-col items-center gap-2 px-4"
	aria-live="polite"
>
	{#each $toasts as t (t.id)}
		<button
			type="button"
			class="pointer-events-auto max-w-sm rounded-[var(--radius-lg)] px-4 py-3 text-sm font-medium shadow-lg"
			class:bg-terracotta={t.kind === 'error'}
			class:text-white={t.kind === 'error' || t.kind === 'success'}
			class:bg-sage={t.kind === 'success'}
			class:bg-white={t.kind === 'info'}
			class:text-ink={t.kind === 'info'}
			onclick={() => toasts.dismiss(t.id)}
		>
			{t.message}
		</button>
	{/each}
</div>
