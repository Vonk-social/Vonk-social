<script lang="ts">
	import type { Snippet } from 'svelte';
	type Variant = 'primary' | 'ghost' | 'amber';

	type Props = {
		variant?: Variant;
		type?: 'button' | 'submit' | 'reset';
		href?: string;
		formaction?: string;
		disabled?: boolean;
		fullWidth?: boolean;
		onclick?: (e: MouseEvent) => void;
		children: Snippet;
	};

	const {
		variant = 'primary',
		type = 'button',
		href,
		formaction,
		disabled = false,
		fullWidth = false,
		onclick,
		children
	}: Props = $props();

	const base =
		'inline-flex items-center justify-center gap-2 px-6 py-3 font-semibold rounded-[var(--radius-button)] transition-colors duration-150 disabled:opacity-50 disabled:cursor-not-allowed focus-visible:outline-2 focus-visible:outline-offset-2';

	const styles: Record<Variant, string> = {
		primary:
			'bg-terracotta text-white hover:bg-terracotta-dark active:bg-terracotta-dark shadow-sm',
		ghost:
			'bg-transparent text-ink border border-border hover:bg-border/40',
		amber: 'bg-amber text-ink hover:brightness-95'
	};

	const className = $derived(`${base} ${styles[variant]} ${fullWidth ? 'w-full' : ''}`);
</script>

{#if href}
	<a class={className} {href} aria-disabled={disabled}>
		{@render children()}
	</a>
{:else}
	<button class={className} {type} {formaction} {disabled} {onclick}>
		{@render children()}
	</button>
{/if}
