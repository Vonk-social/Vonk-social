<script lang="ts">
	/**
	 * Language switcher for the public landing page (and anywhere else a
	 * guest would want to change language before logging in).
	 *
	 * Signed-in users get their locale from `user.locale` via `PATCH
	 * /api/users/me`. Before auth, we persist the choice in a plain cookie
	 * (`vonk_locale`) read by `hooks.server.ts`.
	 *
	 * The control is a native <select> — accessible, keyboard-friendly,
	 * and works without JS if hooks sets the cookie from a form action
	 * (future enhancement; for now we require JS).
	 */
	import { invalidateAll } from '$app/navigation';
	import { SUPPORTED_LOCALES } from '$lib/i18n';

	type Props = { current: string };
	let { current }: Props = $props();

	async function onChange(e: Event) {
		const code = (e.target as HTMLSelectElement).value;
		// 1 year cookie, SameSite=Lax so navigations keep it.
		document.cookie = `vonk_locale=${code}; Path=/; Max-Age=31536000; SameSite=Lax`;
		await invalidateAll(); // re-run +layout.server.ts so locale is re-evaluated
		// Also mirror on <html lang="…">
		document.documentElement.setAttribute('lang', code);
	}
</script>

<label class="inline-flex items-center gap-2 text-sm text-muted">
	<span aria-hidden="true">🌐</span>
	<span class="sr-only">Taal / Language</span>
	<select
		value={current}
		onchange={onChange}
		class="rounded-full border border-border bg-surface px-3 py-1.5 text-ink focus:border-terracotta focus:outline-none"
	>
		{#each SUPPORTED_LOCALES as loc (loc.code)}
			<option value={loc.code}>{loc.flag} {loc.name}</option>
		{/each}
	</select>
</label>
