import { apiFetch } from './core';
import type { PostAuthor } from './posts';

export type ConversationListItem = {
	uuid: string;
	other_user: PostAuthor;
	last_message: string | null;
	last_message_at: string | null;
	unread_count: number;
};

export type DmMessage = {
	uuid: string;
	sender: PostAuthor;
	content: string;
	created_at: string;
	is_mine: boolean;
};

export type MessageListResponse = {
	data: DmMessage[];
	cursor: string | null;
	has_more: boolean;
};

export async function fetchConversations(cookies?: string): Promise<ConversationListItem[]> {
	const res = await apiFetch('/api/dm/conversations', { cookies });
	if (!res.ok) throw new Error(`/api/dm/conversations ${res.status}`);
	return (await res.json()).data;
}

export async function startConversation(
	toUsername: string,
	cookies?: string
): Promise<{ uuid: string }> {
	const res = await apiFetch('/api/dm/conversations', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ to_username: toUsername }),
		cookies
	});
	if (!res.ok) {
		const err = (await res.json().catch(() => ({}))) as { error?: { code?: string; message?: string } };
		if (res.status === 404) {
			throw new Error('Gebruiker niet gevonden. Controleer de gebruikersnaam.');
		}
		throw new Error(err.error?.message ?? `startConversation ${res.status}`);
	}
	return (await res.json()).data;
}

export async function fetchMessages(
	conversationUuid: string,
	opts?: { cursor?: string; limit?: number },
	cookies?: string
): Promise<MessageListResponse> {
	const params = new URLSearchParams();
	if (opts?.cursor) params.set('cursor', opts.cursor);
	if (opts?.limit) params.set('limit', String(opts.limit));
	const qs = params.toString();
	const url = `/api/dm/conversations/${conversationUuid}/messages${qs ? `?${qs}` : ''}`;
	const res = await apiFetch(url, { cookies });
	if (!res.ok) throw new Error(`fetchMessages ${res.status}`);
	return await res.json();
}

export async function sendMessage(
	conversationUuid: string,
	content: string,
	cookies?: string
): Promise<DmMessage> {
	const res = await apiFetch(`/api/dm/conversations/${conversationUuid}/messages`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ content }),
		cookies
	});
	if (!res.ok) {
		const err = (await res.json().catch(() => ({}))) as { error?: { message?: string } };
		throw new Error(err.error?.message ?? `sendMessage ${res.status}`);
	}
	return (await res.json()).data;
}
