import { apiFetch } from './core';
import type { PostAuthor } from './posts';

export type ViewPolicy = 'view_once' | 'view_24h';

export type SnapInboxItem = {
	uuid: string;
	sender: PostAuthor;
	view_policy: ViewPolicy;
	expires_at?: string | null;
	created_at: string;
	viewed_by_me: boolean;
};

export type SentSnapItem = {
	uuid: string;
	recipient: PostAuthor;
	view_policy: ViewPolicy;
	expires_at?: string | null;
	created_at: string;
	viewed_by_them: boolean;
};

export async function sendSnap(
	body: {
		to_username: string;
		media_uuid: string;
		view_policy?: ViewPolicy;
		ephemeral_pubkey?: string;
		nonce?: string;
		ciphertext?: string;
	},
	cookies?: string
): Promise<{ uuid: string }> {
	const res = await apiFetch('/api/snaps', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body),
		cookies
	});
	if (!res.ok) {
		const err = (await res.json().catch(() => ({}))) as { error?: { message?: string } };
		throw new Error(err.error?.message ?? `sendSnap ${res.status}`);
	}
	return (await res.json()).data;
}

export async function fetchSnapInbox(cookies?: string): Promise<SnapInboxItem[]> {
	const res = await apiFetch('/api/snaps/inbox', { cookies });
	if (!res.ok) throw new Error(`/api/snaps/inbox ${res.status}`);
	return (await res.json()).data;
}

export async function fetchSentSnaps(cookies?: string): Promise<SentSnapItem[]> {
	const res = await apiFetch('/api/snaps/sent', { cookies });
	if (!res.ok) throw new Error(`/api/snaps/sent ${res.status}`);
	return (await res.json()).data;
}

/**
 * Consume a view-once / view-24h snap. Returns the presigned media URL,
 * which expires in 30 seconds. Throws if already viewed or expired.
 */
export async function viewSnap(
	uuid: string,
	cookies?: string
): Promise<{ url: string; expires_at: string }> {
	const res = await apiFetch(`/api/snaps/${uuid}/view`, { cookies });
	if (!res.ok) {
		const err = (await res.json().catch(() => ({}))) as {
			error?: { code?: string; message?: string };
		};
		const e = new Error(err.error?.message ?? `viewSnap ${res.status}`) as Error & {
			code?: string;
			status: number;
		};
		e.code = err.error?.code;
		e.status = res.status;
		throw e;
	}
	return (await res.json()).data;
}

export async function unsendSnap(uuid: string, cookies?: string): Promise<void> {
	const res = await apiFetch(`/api/snaps/${uuid}`, { method: 'DELETE', cookies });
	if (!res.ok) throw new Error(`unsendSnap ${res.status}`);
}
