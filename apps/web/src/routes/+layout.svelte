<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import type { Snippet } from 'svelte';
	import BottomNav from '$lib/components/ui/BottomNav.svelte';
	import { t } from '$lib/i18n';
	import type { LayoutProps } from './$types';

	let { data, children }: LayoutProps & { children: Snippet } = $props();

	const locale = $derived(data.user?.locale ?? data.locale ?? 'nl');

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

	/**
	 * Show the mini legal footer on authenticated pages (not landing, not
	 * camera, not privacy/terms pages themselves).
	 */
	const showMiniFooter = $derived.by(() => {
		const p = $page.url.pathname;
		if (p === '/' || p === '/privacy' || p === '/terms') return false;
		if (p.startsWith('/camera')) return false;
		return true;
	});
</script>

<div class="min-h-screen" class:with-bottom-nav={showBottomNav}>
	{@render children()}

	{#if showMiniFooter}
		<footer class="px-6 pb-8 pt-4">
			<div class="mx-auto flex max-w-4xl items-center justify-center gap-3 text-xs text-muted">
				<a
					href="/privacy"
					class="underline decoration-border underline-offset-4 hover:text-ink"
				>
					{t('footer.privacy', locale)}
				</a>
				<span class="text-border">|</span>
				<a
					href="/terms"
					class="underline decoration-border underline-offset-4 hover:text-ink"
				>
					{t('footer.terms', locale)}
				</a>
			</div>
		</footer>
	{/if}
</div>

{#if showBottomNav && data.user}
	<BottomNav user={data.user} unreadCount={data.unreadCount} />
{/if}
