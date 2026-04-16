<script lang="ts">
	/**
	 * Growth CTA banner shown prominently on /home when the user has
	 * fewer than 30 total connections (followers + following). Disappears
	 * once they cross the threshold, or they can dismiss it for the session.
	 *
	 * Three actions:
	 *   1. "Nodig vrienden uit" → /invite (email + handle-match)
	 *   2. "Ontdek mensen" → /discover
	 *   3. Dismiss ✕ → hides for the browser session
	 */

	type Props = {
		connectionCount: number;
		/** e.g. "dimitry" — used to build the shareable profile link */
		username: string;
	};
	const { connectionCount, username }: Props = $props();

	let dismissed = $state(false);

	const threshold = 30;
	const show = $derived(connectionCount < threshold && !dismissed);

	function dismiss() {
		dismissed = true;
		try {
			sessionStorage.setItem('vonk:invite-banner-dismissed', '1');
		} catch {
			// private browsing
		}
	}

	// Restore dismiss state from sessionStorage on mount.
	import { onMount } from 'svelte';
	onMount(() => {
		try {
			if (sessionStorage.getItem('vonk:invite-banner-dismissed') === '1') {
				dismissed = true;
			}
		} catch {
			// private browsing
		}
	});

	const progress = $derived(Math.min(100, Math.round((connectionCount / threshold) * 100)));
</script>

{#if show}
	<section
		class="vonk-spark relative mb-6 overflow-hidden rounded-2xl p-5 text-white shadow-lg"
	>
		<!-- Dismiss button -->
		<button
			onclick={dismiss}
			class="absolute top-3 right-3 flex h-7 w-7 items-center justify-center rounded-full bg-white/20 text-white/80 transition-colors hover:bg-white/30"
			aria-label="Sluiten"
		>✕</button>

		<h2 class="mb-1 font-display text-lg font-bold">
			{#if connectionCount === 0}
				Vonk is leuker met vrienden!
			{:else}
				Je hebt {connectionCount} connectie{connectionCount === 1 ? '' : 's'} — bijna daar!
			{/if}
		</h2>
		<p class="mb-4 text-sm text-white/85">
			{#if connectionCount === 0}
				Je feed is nog leeg. Nodig je vrienden uit of zoek mensen die je kent.
			{:else}
				Nodig nog wat mensen uit zodat je feed tot leven komt. Vonk groeit enkel via vrienden — geen reclame, geen algoritme.
			{/if}
		</p>

		<!-- Progress bar -->
		<div class="mb-4 flex items-center gap-3">
			<div class="h-2 flex-1 overflow-hidden rounded-full bg-white/20">
				<div
					class="h-full rounded-full bg-white/70 transition-all"
					style="width: {progress}%"
				></div>
			</div>
			<span class="text-xs font-semibold text-white/70">{connectionCount}/{threshold}</span>
		</div>

		<div class="flex flex-wrap gap-2">
			<a
				href="/invite"
				class="inline-flex items-center gap-2 rounded-full bg-white px-5 py-2.5 text-sm font-bold text-terracotta shadow transition-transform hover:scale-105"
			>
				✉️ Nodig vrienden uit
			</a>
			<a
				href="/discover"
				class="inline-flex items-center gap-2 rounded-full border border-white/40 px-5 py-2.5 text-sm font-semibold text-white transition-colors hover:bg-white/10"
			>
				🔍 Ontdek mensen
			</a>
		</div>

		<!-- Share link hint -->
		<p class="mt-3 text-xs text-white/60">
			Of deel je profiel: <span class="font-mono text-white/80">vonk.social/u/{username}</span>
		</p>
	</section>
{/if}
