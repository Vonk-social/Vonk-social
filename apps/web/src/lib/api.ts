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
};

export type MeProfile = SessionUser;

export type ApiError = {
	error: { code: string; message: string };
};

function baseUrl(): string {
	return browser ? '' : SERVER_API;
}

export async function apiFetch(
	path: string,
	init: RequestInit & { cookies?: string } = {}
): Promise<Response> {
	const { cookies, ...rest } = init;
	const headers = new Headers(rest.headers ?? {});
	if (!browser && cookies) headers.set('cookie', cookies);
	return fetch(`${baseUrl()}${path}`, {
		...rest,
		headers,
		credentials: browser ? 'include' : undefined
	});
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
