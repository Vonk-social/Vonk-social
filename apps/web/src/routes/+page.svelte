<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import GoogleButton from '$lib/components/GoogleButton.svelte';
	import VonkLogo from '$lib/components/VonkLogo.svelte';
	import LanguageSwitcher from '$lib/components/ui/LanguageSwitcher.svelte';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
	const locale = $derived(data.user?.locale ?? data.locale ?? 'nl');

	onMount(() => {
		if (data.user?.needs_onboarding) goto('/onboarding/username');
		else if (data.user) goto('/home');
	});
</script>

<svelte:head>
	<title>Vonk — {t('landing.hero.title', locale)}</title>
</svelte:head>

<!-- ── Top bar with language switcher ───────────────────────── -->
<div class="absolute top-4 right-4 z-10">
	<LanguageSwitcher current={locale} />
</div>

<!-- ── Hero ──────────────────────────────────────────────────── -->
<section class="px-6 pt-20 pb-16 md:pb-24">
	<div class="mx-auto flex max-w-3xl flex-col items-center text-center">
		<VonkLogo size={96} alt="Vonk" />

		<span
			class="mt-6 inline-flex items-center gap-2 rounded-full bg-surface/70 px-4 py-1.5 text-sm font-semibold text-terracotta-dark shadow-sm"
		>
			{t('landing.badge', locale)}
		</span>

		<h1 class="mt-6 font-display text-4xl font-extrabold leading-tight text-ink md:text-6xl">
			{t('landing.hero.title', locale)}
			<span aria-hidden="true" class="ml-1 inline-block animate-wave">✌️</span>
		</h1>

		<p class="mt-6 max-w-xl text-lg text-muted">
			{t('landing.hero.subtitle', locale)}
		</p>

		<div class="mt-10 flex w-full max-w-xs flex-col gap-3">
			<GoogleButton label={t('landing.cta.login', locale)} />
			<a
				href="#why"
				class="text-center text-sm font-medium text-muted underline decoration-border underline-offset-4 hover:text-ink"
			>
				{t('landing.cta.learn', locale)}
			</a>
		</div>
	</div>
</section>

<!-- ── Waarom Vonk? ─────────────────────────────────────────── -->
<section id="why" class="scroll-mt-12 px-6 py-16">
	<div class="mx-auto max-w-4xl">
		<header class="mb-10 text-center">
			<h2 class="font-display text-3xl font-extrabold text-ink md:text-4xl">
				{t('landing.why.title', locale)}
			</h2>
			<p class="mx-auto mt-4 max-w-2xl text-muted">
				{t('landing.why.intro', locale)}
			</p>
		</header>

		<div class="grid gap-5 md:grid-cols-2">
			<article class="vonk-card">
				<div
					class="mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-terracotta/10 text-terracotta"
					aria-hidden="true"
				>
					<svg viewBox="0 0 24 24" class="h-6 w-6" fill="none" stroke="currentColor" stroke-width="2">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M12 3l8 4v5c0 4.4-3.4 8.2-8 9-4.6-.8-8-4.6-8-9V7l8-4z"
						/>
					</svg>
				</div>
				<h3 class="font-display text-lg font-bold text-ink">
					{t('landing.pillar1.title', locale)}
				</h3>
				<p class="mt-2 text-muted">{t('landing.pillar1.body', locale)}</p>
			</article>

			<article class="vonk-card">
				<div
					class="mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-sage/20 text-sage"
					aria-hidden="true"
				>
					<svg viewBox="0 0 24 24" class="h-6 w-6" fill="none" stroke="currentColor" stroke-width="2">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M8 9l-4 3 4 3M16 9l4 3-4 3M14 5l-4 14"
						/>
					</svg>
				</div>
				<h3 class="font-display text-lg font-bold text-ink">
					{t('landing.pillar2.title', locale)}
				</h3>
				<p class="mt-2 text-muted">{t('landing.pillar2.body', locale)}</p>
			</article>

			<article class="vonk-card">
				<div
					class="mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-amber/20 text-amber"
					aria-hidden="true"
				>
					<svg viewBox="0 0 24 24" class="h-6 w-6" fill="none" stroke="currentColor" stroke-width="2">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M12 21s-7-4.5-7-10a5 5 0 019-3 5 5 0 019 3c0 5.5-7 10-7 10h-4z"
						/>
					</svg>
				</div>
				<h3 class="font-display text-lg font-bold text-ink">
					{t('landing.pillar3.title', locale)}
				</h3>
				<p class="mt-2 text-muted">{t('landing.pillar3.body', locale)}</p>
			</article>

			<article class="vonk-card">
				<div
					class="vonk-spark mb-3 flex h-10 w-10 items-center justify-center rounded-full text-white"
					aria-hidden="true"
				>
					<svg viewBox="0 0 24 24" class="h-6 w-6" fill="currentColor">
						<path d="M13 3l-2 6h5l-7 12 2-8H6l7-10z" />
					</svg>
				</div>
				<h3 class="font-display text-lg font-bold text-ink">
					{t('landing.pillar4.title', locale)}
				</h3>
				<p class="mt-2 text-muted">{t('landing.pillar4.body', locale)}</p>
			</article>
		</div>
	</div>
</section>

<!-- ── Distributed network ─────────────────────────────────── -->
<section class="px-6 py-12">
	<div class="mx-auto max-w-3xl">
		<article class="vonk-spark rounded-[var(--radius-card)] p-8 text-white shadow-lg">
			<h2 class="font-display text-2xl font-bold">
				🌐 {t('landing.distributed.title', locale)}
			</h2>
			<p class="mt-3 text-white/85">{t('landing.distributed.body', locale)}</p>
			<div class="mt-5">
				<a
					href="/host"
					class="inline-flex items-center gap-2 rounded-full bg-white px-6 py-3 font-bold text-terracotta shadow transition-transform hover:scale-105"
				>
					🖥️ {t('landing.distributed.cta', locale)}
				</a>
			</div>
		</article>
	</div>
</section>

<!-- ── Open finances ────────────────────────────────────────── -->
<section class="px-6 py-12">
	<div class="mx-auto max-w-3xl">
		<article class="vonk-card border-2 border-terracotta/20">
			<h2 class="font-display text-2xl font-bold text-ink">
				{t('landing.finances.title', locale)}
			</h2>
			<p class="mt-3 text-muted">{t('landing.finances.body', locale)}</p>
		</article>
	</div>
</section>

<!-- ── Final CTA ────────────────────────────────────────────── -->
<section class="px-6 py-16">
	<div class="mx-auto flex max-w-2xl flex-col items-center text-center">
		<h2 class="font-display text-3xl font-extrabold text-ink md:text-4xl">
			{t('landing.cta2.title', locale)}
		</h2>
		<p class="mt-4 max-w-xl text-muted">
			{t('landing.cta2.body', locale)}
		</p>
		<div class="mt-8 w-full max-w-xs">
			<GoogleButton label={t('landing.cta.login', locale)} />
		</div>
	</div>
</section>

<!-- ── Footer ───────────────────────────────────────────────── -->
<footer class="border-t border-border px-6 py-8">
	<div
		class="mx-auto flex max-w-4xl flex-col items-center justify-between gap-3 text-xs text-muted md:flex-row"
	>
		<span>{t('landing.footer.license', locale)}</span>
		<a
			href="https://github.com/Vonk-social/Vonk-social"
			rel="noopener"
			class="underline decoration-border underline-offset-4 hover:text-ink"
		>
			{t('landing.footer.source', locale)}
		</a>
		<span>{t('landing.footer.europe', locale)}</span>
	</div>
</footer>

<style>
	@keyframes wave {
		0%,
		60%,
		100% {
			transform: rotate(0deg);
		}
		10% {
			transform: rotate(14deg);
		}
		20% {
			transform: rotate(-8deg);
		}
		30% {
			transform: rotate(14deg);
		}
		40% {
			transform: rotate(-4deg);
		}
		50% {
			transform: rotate(10deg);
		}
	}
	.animate-wave {
		display: inline-block;
		transform-origin: 70% 70%;
		animation: wave 2.8s ease-in-out infinite;
	}
	@media (prefers-reduced-motion: reduce) {
		.animate-wave {
			animation: none;
		}
	}
</style>
