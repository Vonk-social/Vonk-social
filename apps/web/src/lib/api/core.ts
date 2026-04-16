/**
 * Thin wrapper around the Vonk HTTP API.
 *
 * On the server (SvelteKit SSR) we call the backend via `http://localhost:8080`
 * directly — bypassing the Vite proxy — because server-to-server traffic
 * inside the same dev machine is faster and avoids the proxy round-trip.
 * Cookies are forwarded manually by the caller.
 *
 * On the browser we call relative `/api/...` paths which the Vite dev server
 * (or nginx in prod) proxies to the backend. `credentials: 'include'` ensures
 * cookies are sent.
 */

import { browser } from '$app/environment';

// Server-side base URL. Bypasses the Vite/nginx proxy so hooks.server.ts can
// talk to the backend directly. Guarded against browser evaluation of
// `process.env` (which would throw ReferenceError on `process`).
const SERVER_API =
	(typeof process !== 'undefined' && process.env?.API_INTERNAL_URL) || 'http://localhost:8080';

export type SessionUser = {
	uuid: string;
	username: string;
	display_name: string;
	email?: string | null;
	email_verified: boolean;
	bio: string;
	avatar_url?: string | null;
	banner_url?: string | null;
	location_city?: string | null;
	location_country?: string | null;
	locale: string;
	is_private: boolean;
	needs_onboarding: boolean;
	created_at: string;
	handle_instagram?: string | null;
	handle_twitter?: string | null;
	handle_snapchat?: string | null;
	handle_telegram?: string | null;
	handle_bluesky?: string | null;
	handle_mastodon?: string | null;
	handle_website?: string | null;
	public_key?: string | null;
};

export type MeProfile = SessionUser;

export type ApiError = {
	error: { code: string; message: string };
};

function baseUrl(): string {
	return browser ? '' : SERVER_API;
}

/**
 * Client-side single-flight guard so many concurrent 401s (e.g. a feed that
 * fires like/reply/follow together) produce exactly one `/api/auth/refresh`
 * request rather than a stampede.
 */
let refreshInFlight: Promise<boolean> | null = null;

async function browserRefreshOnce(): Promise<boolean> {
	if (!browser) return false;
	if (!refreshInFlight) {
		refreshInFlight = fetch('/api/auth/refresh', {
			method: 'POST',
			credentials: 'include'
		})
			.then((r) => r.ok)
			.catch(() => false)
			.finally(() => {
				// Null out after the microtask so pipelined callers all see the
				// same result before the next refresh becomes possible.
				setTimeout(() => {
					refreshInFlight = null;
				}, 0);
			});
	}
	return refreshInFlight;
}

function buildInit(init: RequestInit & { cookies?: string }): RequestInit {
	const { cookies: _unused, ...rest } = init;
	const headers = new Headers(rest.headers ?? {});
	if (!browser && init.cookies) headers.set('cookie', init.cookies);
	return {
		...rest,
		headers,
		credentials: browser ? 'include' : undefined
	};
}

export async function apiFetch(
	path: string,
	init: RequestInit & { cookies?: string } = {}
): Promise<Response> {
	const url = `${baseUrl()}${path}`;
	const res = await fetch(url, buildInit(init));

	// Browser-only: transparently recover from access-token expiry. A single
	// /api/auth/refresh produces a new vonk_access cookie; we retry the
	// original call once. Never runs on the server (hooks.server.ts already
	// handles refresh there and would otherwise loop).
	if (browser && res.status === 401 && !path.startsWith('/api/auth/')) {
		const refreshed = await browserRefreshOnce();
		if (refreshed) return fetch(url, buildInit(init));
	}
	return res;
}

/** Fetch `/api/users/me`. Returns null on 401. */
export async function fetchMe(cookies?: string): Promise<SessionUser | null> {
	const res = await apiFetch('/api/users/me', { method: 'GET', cookies });
	if (res.status === 401 || res.status === 404) return null;
	if (!res.ok) throw new Error(`fetchMe ${res.status}`);
	const body = (await res.json()) as { data: SessionUser };
	return body.data;
}

/** PATCH the current user. Returns updated profile or throws with { code, message }. */
export async function patchMe(
	patch: Partial<{
		username: string;
		display_name: string;
		bio: string;
		location_city: string;
		location_country: string;
		locale: string;
		is_private: boolean;
		finish_onboarding: boolean;
		handle_instagram: string;
		handle_twitter: string;
		handle_snapchat: string;
		handle_telegram: string;
		handle_bluesky: string;
		handle_mastodon: string;
		handle_website: string;
		public_key: string;
	}>,
	cookies?: string
): Promise<SessionUser> {
	const res = await apiFetch('/api/users/me', {
		method: 'PATCH',
		cookies,
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(patch)
	});
	if (!res.ok) {
		const err = (await res.json().catch(() => ({}))) as Partial<ApiError>;
		const code = err.error?.code ?? 'unknown';
		const msg = err.error?.message ?? `HTTP ${res.status}`;
		const e = new Error(msg) as Error & { code: string };
		e.code = code;
		throw e;
	}
	const body = (await res.json()) as { data: SessionUser };
	return body.data;
}

/** Check username availability. */
export async function checkUsername(
	q: string,
	cookies?: string
): Promise<{ available: boolean; reason?: string }> {
	const res = await apiFetch(`/api/users/check-username?q=${encodeURIComponent(q)}`, {
		method: 'GET',
		cookies
	});
	if (!res.ok) return { available: false, reason: 'api_error' };
	const body = (await res.json()) as { data: { available: boolean; reason?: string } };
	return body.data;
}

/** Upload an avatar. */
export async function uploadAvatar(file: File): Promise<{ avatar_url: string }> {
	const fd = new FormData();
	fd.append('file', file);
	const res = await apiFetch('/api/users/me/avatar', {
		method: 'POST',
		body: fd
	});
	if (!res.ok) {
		const err = (await res.json().catch(() => ({}))) as Partial<ApiError>;
		throw new Error(err.error?.message ?? `HTTP ${res.status}`);
	}
	const body = (await res.json()) as { data: { avatar_url: string } };
	return body.data;
}
