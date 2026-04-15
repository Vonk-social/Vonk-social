<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import GoogleButton from '$lib/components/GoogleButton.svelte';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
	const locale = $derived(data.user?.locale ?? 'nl');

	onMount(() => {
		if (data.user?.needs_onboarding) goto('/onboarding/username');
		else if (data.user) goto('/home');
	});
</script>

<svelte:head>
	<title>Vonk — {t('landing.hero.title', locale)}</title>
</svelte:head>

<main class="mx-auto flex min-h-screen max-w-3xl flex-col items-center justify-center px-6 py-16">
	<span
		class="mb-6 inline-flex items-center gap-2 rounded-full bg-white/70 px-4 py-1.5 text-sm font-semibold text-terracotta-dark shadow-sm"
	>
		{t('landing.badge', locale)}
	</span>

	<h1 class="text-center font-display text-5xl font-extrabold leading-tight text-ink md:text-6xl">
		{t('landing.hero.title', locale)}
	</h1>

	<p class="mt-6 max-w-xl text-center text-lg text-muted">
		{t('landing.hero.subtitle', locale)}
	</p>

	<div class="mt-10 flex w-full max-w-xs flex-col gap-3">
		<GoogleButton label={t('landing.cta.login', locale)} />
		<a
			href="https://github.com/Vonk-social/Vonk-social"
			class="text-center text-sm font-medium text-muted underline decoration-border underline-offset-4 hover:text-ink"
		>
			{t('landing.cta.learn', locale)}
		</a>
	</div>

	<footer class="mt-16 text-xs text-muted">
		Vonk — AGPL-3.0 · {t('landing.badge', locale)}
	</footer>
</main>
