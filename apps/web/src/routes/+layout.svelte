<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import type { Snippet } from 'svelte';
	import BottomNav from '$lib/components/ui/BottomNav.svelte';
	import type { LayoutProps } from './$types';

	let { data, children }: LayoutProps & { children: Snippet } = $props();

	/**
	 * Where the bottom nav should show. Rules:
	 * - Only for logged-in users with onboarding done.
	 * - Hidden on /camera (full-screen capture) and /onboarding/* (wizard).
	 * - Hidden on the landing route ('/') even when briefly logged-in before
	 *   the onMount redirect fires.
	 */
	const showBottomNav = $derived.by(() => {
		if (!data.user || data.user.needs_onboarding) return false;
		const p = $page.url.pathname;
		if (p === '/' || p.startsWith('/onboarding') || p.startsWith('/camera')) return false;
		if (p.startsWith('/login')) return false;
		return true;
	});
</script>

<div class="min-h-screen" class:with-bottom-nav={showBottomNav}>
	{@render children()}
</div>

{#if showBottomNav && data.user}
	<BottomNav user={data.user} unreadCount={data.unreadCount} />
{/if}
