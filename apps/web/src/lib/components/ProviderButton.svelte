<script lang="ts">
	/**
	 * Generic OAuth provider button. Used for GitHub + Apple, styled
	 * to match GoogleButton so the login page looks cohesive.
	 */
	type Props = {
		label: string;
		href: string;
		provider: 'github' | 'apple';
	};
	const { label, href, provider }: Props = $props();
</script>

<a
	{href}
	class="inline-flex w-full items-center justify-center gap-3 rounded-[var(--radius-button)]
	       border border-border bg-white px-6 py-3 font-semibold text-ink shadow-sm
	       transition-colors hover:bg-border/30 focus-visible:outline-2 focus-visible:outline-offset-2
	       dark:bg-ink dark:text-cream dark:border-ink/30"
>
	{#if provider === 'github'}
		<svg aria-hidden="true" viewBox="0 0 24 24" class="h-5 w-5" fill="currentColor">
			<path
				d="M12 .5C5.37.5 0 5.87 0 12.5c0 5.3 3.44 9.8 8.21 11.39.6.11.82-.26.82-.58v-2c-3.34.73-4.04-1.41-4.04-1.41-.55-1.39-1.34-1.76-1.34-1.76-1.09-.75.08-.73.08-.73 1.21.08 1.85 1.24 1.85 1.24 1.07 1.84 2.81 1.31 3.5 1 .11-.78.42-1.31.76-1.61-2.66-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.12-.3-.54-1.52.12-3.17 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 0 1 6 0c2.29-1.55 3.3-1.23 3.3-1.23.66 1.65.24 2.87.12 3.17.77.84 1.24 1.91 1.24 3.22 0 4.61-2.81 5.63-5.49 5.92.43.37.81 1.1.81 2.22v3.29c0 .32.22.69.83.58A12 12 0 0 0 24 12.5c0-6.63-5.37-12-12-12z"
			/>
		</svg>
	{:else}
		<!-- Apple logo -->
		<svg aria-hidden="true" viewBox="0 0 24 24" class="h-5 w-5" fill="currentColor">
			<path
				d="M17.05 20.28c-.98.95-2.05.88-3.08.44-1.09-.45-2.09-.47-3.24 0-1.44.62-2.2.44-3.06-.44C2.79 15.25 3.51 7.59 9.05 7.31c1.35.07 2.29.74 3.08.8 1.18-.24 2.31-.93 3.57-.84 1.51.12 2.65.72 3.4 1.8-3.12 1.87-2.38 5.98.48 7.13-.57 1.5-1.31 2.99-2.53 4.08zM12.03 7.25c-.15-2.23 1.66-4.07 3.74-4.25.29 2.58-2.34 4.5-3.74 4.25z"
			/>
		</svg>
	{/if}
	{label}
</a>
