<script lang="ts">
	/**
	 * Instagram-style bottom tab bar. Fixed to the viewport bottom, always
	 * visible on authenticated pages so the four primary destinations +
	 * camera are one tap away while scrolling.
	 *
	 * Pages that embed this must reserve bottom padding via the global
	 * `.with-bottom-nav` utility in app.css so content doesn't hide under it.
	 * /camera and /onboarding/* skip rendering entirely (full-screen flows).
	 */
	import { page } from '$app/stores';
	import Avatar from './Avatar.svelte';
	import type { SessionUser } from '$lib/api/core';

	type Props = { user: SessionUser };
	let { user }: Props = $props();

	const path = $derived($page.url.pathname);
	const profilePath = $derived(`/u/${user.username}`);

	const homeActive = $derived(path === '/home');
	const discoverActive = $derived(path.startsWith('/discover'));
	const dmActive = $derived(path.startsWith('/dm'));
	const profileActive = $derived(path.startsWith('/u/'));
</script>

<nav
	aria-label="Hoofdnavigatie"
	class="fixed right-0 bottom-0 left-0 z-40 border-t border-border bg-surface/95 backdrop-blur"
	style="padding-bottom: env(safe-area-inset-bottom, 0px);"
>
	<ul class="mx-auto flex max-w-xl items-center justify-around px-2 py-1">
		<!-- Home -->
		<li class="flex-1">
			<a
				href="/home"
				aria-label="Home"
				aria-current={homeActive ? 'page' : undefined}
				class="flex flex-col items-center gap-0.5 rounded-lg py-2 text-xs font-medium transition-colors"
				class:text-terracotta={homeActive}
				class:text-muted={!homeActive}
			>
				<svg
					viewBox="0 0 24 24"
					class="h-6 w-6"
					fill={homeActive ? 'currentColor' : 'none'}
					stroke="currentColor"
					stroke-width="2"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M3 11.5L12 4l9 7.5V20a1 1 0 01-1 1h-5v-6H9v6H4a1 1 0 01-1-1v-8.5z"
					/>
				</svg>
				<span>Home</span>
			</a>
		</li>

		<!-- Discover -->
		<li class="flex-1">
			<a
				href="/discover"
				aria-label="Zoek"
				aria-current={discoverActive ? 'page' : undefined}
				class="flex flex-col items-center gap-0.5 rounded-lg py-2 text-xs font-medium transition-colors"
				class:text-terracotta={discoverActive}
				class:text-muted={!discoverActive}
			>
				<svg
					viewBox="0 0 24 24"
					class="h-6 w-6"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<circle cx="11" cy="11" r="7" />
					<path stroke-linecap="round" d="M20 20l-3.5-3.5" />
				</svg>
				<span>Zoek</span>
			</a>
		</li>

		<!-- Camera (prominent centre tab) -->
		<li class="flex-1">
			<a
				href="/camera"
				aria-label="Camera"
				class="vonk-spark -mt-4 mx-auto flex h-14 w-14 items-center justify-center rounded-full text-white shadow-lg transition-transform hover:scale-105"
			>
				<svg viewBox="0 0 24 24" class="h-7 w-7" fill="currentColor" aria-hidden="true">
					<path
						d="M9 4l2-2h2l2 2h4a2 2 0 012 2v11a2 2 0 01-2 2H5a2 2 0 01-2-2V6a2 2 0 012-2h4zm3 4a5 5 0 100 10 5 5 0 000-10zm0 2.5a2.5 2.5 0 110 5 2.5 2.5 0 010-5z"
					/>
				</svg>
			</a>
		</li>

		<!-- DM / Snaps -->
		<li class="flex-1">
			<a
				href="/dm"
				aria-label="Snaps"
				aria-current={dmActive ? 'page' : undefined}
				class="flex flex-col items-center gap-0.5 rounded-lg py-2 text-xs font-medium transition-colors"
				class:text-terracotta={dmActive}
				class:text-muted={!dmActive}
			>
				<svg
					viewBox="0 0 24 24"
					class="h-6 w-6"
					fill={dmActive ? 'currentColor' : 'none'}
					stroke="currentColor"
					stroke-width="2"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M21 12c0 4.5-4 8-9 8a9.7 9.7 0 01-3.8-.75L3 21l1.25-4.5A8 8 0 013 12c0-4.4 4-8 9-8s9 3.6 9 8z"
					/>
				</svg>
				<span>Snaps</span>
			</a>
		</li>

		<!-- Profile -->
		<li class="flex-1">
			<a
				href={profilePath}
				aria-label="Mijn profiel"
				aria-current={profileActive ? 'page' : undefined}
				class="flex flex-col items-center gap-0.5 rounded-lg py-2 text-xs font-medium transition-colors"
				class:text-terracotta={profileActive}
				class:text-muted={!profileActive}
			>
				<span class="relative inline-block">
					<Avatar url={user.avatar_url} name={user.display_name} size={26} />
					{#if profileActive}
						<span
							class="absolute -inset-0.5 rounded-full ring-2 ring-terracotta"
							aria-hidden="true"
						></span>
					{/if}
				</span>
				<span>Profiel</span>
			</a>
		</li>
	</ul>
</nav>
