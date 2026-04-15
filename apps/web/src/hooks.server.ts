import type { Handle } from '@sveltejs/kit';
import { fetchMe } from '$lib/api';
import { isLocale, pickLocaleFromHeader } from '$lib/i18n';

const API_INTERNAL = 'http://localhost:8080';

/**
 * Populate `event.locals.user` + `event.locals.locale` on every request.
 *
 * Failures are logged and swallowed: a broken hooks layer must never turn a
 * normal page load into a 500. If we can't resolve the user, we just leave
 * `locals.user = null` and let downstream route guards redirect to /login.
 */
export const handle: Handle = async ({ event, resolve }) => {
	event.locals.user = null;

	// ── Locale resolution ──────────────────────────────────────
	// Precedence: explicit cookie → Accept-Language → 'nl'.
	// When a user logs in, their user.locale overrides this downstream.
	const cookieLocale = event.cookies.get('vonk_locale');
	const headerLocale = pickLocaleFromHeader(event.request.headers.get('accept-language'));
	event.locals.locale = isLocale(cookieLocale) ? cookieLocale : headerLocale;

	const incomingCookies = event.request.headers.get('cookie') ?? '';
	if (incomingCookies.includes('vonk_access') || incomingCookies.includes('vonk_refresh')) {
		try {
			let user = await fetchMe(incomingCookies).catch((e) => {
				console.warn('[hooks] fetchMe failed:', (e as Error).message);
				return null;
			});

			if (!user && incomingCookies.includes('vonk_refresh')) {
				const refresh = await fetch(`${API_INTERNAL}/api/auth/refresh`, {
					method: 'POST',
					headers: { cookie: incomingCookies }
				}).catch((e) => {
					console.warn('[hooks] refresh fetch failed:', (e as Error).message);
					return null;
				});

				if (refresh?.ok) {
					copySetCookies(refresh, event);

					const newAccess = event.cookies.get('vonk_access');
					if (newAccess) {
						user = await fetchMe(
							mergeCookie(incomingCookies, 'vonk_access', newAccess)
						).catch(() => null);
					}
				}
			}

			if (user) {
				event.locals.user = user;
				if (user.locale && isLocale(user.locale)) {
					event.locals.locale = user.locale;
				}
			}
		} catch (err) {
			console.error('[hooks] unexpected error (swallowed):', err);
		}
	}

	return resolve(event, {
		transformPageChunk: ({ html }) =>
			html.replace('%sveltekit.lang%', event.locals.locale ?? 'nl')
	});
};

/** Copy Set-Cookie headers from an upstream `fetch` response into the SvelteKit cookie jar. */
function copySetCookies(upstream: Response, event: Parameters<Handle>[0]['event']) {
	const setCookies =
		typeof upstream.headers.getSetCookie === 'function'
			? upstream.headers.getSetCookie()
			: ([] as string[]);
	for (const raw of setCookies) {
		const parts = raw.split(';').map((s) => s.trim());
		const [assign, ...attrs] = parts;
		const eq = assign.indexOf('=');
		if (eq <= 0) continue;
		const name = assign.slice(0, eq);
		const value = assign.slice(eq + 1);

		const opts: Parameters<typeof event.cookies.set>[2] = {
			path: '/',
			httpOnly: true,
			sameSite: 'lax'
		};
		for (const attr of attrs) {
			const [k, v = ''] = attr.split('=');
			switch (k.toLowerCase()) {
				case 'path':
					opts.path = v || '/';
					break;
				case 'max-age': {
					const n = Number(v);
					if (!Number.isNaN(n)) opts.maxAge = n;
					break;
				}
				case 'domain':
					if (v) opts.domain = v;
					break;
				case 'secure':
					opts.secure = true;
					break;
				case 'samesite': {
					const low = (v || 'lax').toLowerCase();
					if (low === 'lax' || low === 'strict' || low === 'none') opts.sameSite = low;
					break;
				}
				case 'httponly':
					opts.httpOnly = true;
					break;
			}
		}

		try {
			event.cookies.set(name, value, opts);
		} catch (e) {
			console.warn(`[hooks] cookies.set(${name}) failed:`, (e as Error).message);
		}
	}
}

/** Merge/override a single cookie in a raw Cookie header string. */
function mergeCookie(raw: string, name: string, value: string): string {
	const pairs = raw
		.split(';')
		.map((s) => s.trim())
		.filter((s) => s && !s.toLowerCase().startsWith(`${name.toLowerCase()}=`));
	pairs.push(`${name}=${value}`);
	return pairs.join('; ');
}
