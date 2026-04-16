import type { PageServerLoad } from './$types';

const API_INTERNAL_URL =
	(typeof process !== 'undefined' && process.env?.API_INTERNAL_URL) ||
	'http://localhost:8080';

/**
 * Ask the API which OAuth providers have credentials loaded so we only
 * render the buttons that will actually work. Fails open (Google only)
 * if /api/health is unreachable — better to show one button than crash
 * the page.
 */
export const load: PageServerLoad = async ({ fetch }) => {
	let flags = {
		google: true,
		github: false,
		apple: false
	};
	try {
		const res = await fetch(`${API_INTERNAL_URL}/api/health`);
		if (res.ok) {
			const json = await res.json();
			flags = {
				google: Boolean(json.google_oauth_configured),
				github: Boolean(json.github_oauth_configured),
				apple: Boolean(json.apple_oauth_configured)
			};
		}
	} catch {
		// keep defaults
	}
	return { providers: flags };
};
