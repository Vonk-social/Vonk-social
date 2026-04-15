<script lang="ts">
	import { page } from '$app/stores';
	import type { Snippet } from 'svelte';
	import type { LayoutProps } from './$types';
	import VonkLogo from '$lib/components/VonkLogo.svelte';
	import { t } from '$lib/i18n';

	let { data, children }: LayoutProps & { children: Snippet } = $props();
	const locale = $derived(data.user?.locale ?? 'nl');

	const steps = $derived([
		{ path: '/onboarding/username', label: t('onboarding.step_username', locale) },
		{ path: '/onboarding/avatar', label: t('onboarding.step_avatar', locale) },
		{ path: '/onboarding/friends', label: t('onboarding.step_friends', locale) }
	]);

	const current = $derived(
		Math.max(
			0,
			steps.findIndex((s) => $page.url.pathname.startsWith(s.path))
		)
	);
</script>

<main class="mx-auto flex min-h-screen max-w-xl flex-col px-6 py-10">
	<header class="mb-8">
		<div class="mb-4 flex items-center gap-3">
			<VonkLogo size={36} />
			<span class="font-display text-xl font-bold text-ink">Vonk</span>
		</div>
		<h1 class="font-display text-2xl font-bold text-ink">{t('onboarding.title', locale)}</h1>
		<div class="mt-4 flex items-center gap-2">
			{#each steps as s, i}
				<span
					class="h-2 flex-1 rounded-full transition-colors"
					class:bg-terracotta={i <= current}
					class:bg-border={i > current}
					aria-hidden="true"
				></span>
			{/each}
		</div>
		<p class="mt-2 text-sm text-muted">
			{t('onboarding.step_of', locale)
				.replace('{current}', String(current + 1))
				.replace('{total}', String(steps.length))}
			— {steps[current].label}
		</p>
	</header>

	<section class="flex-1">
		{@render children()}
	</section>
</main>
