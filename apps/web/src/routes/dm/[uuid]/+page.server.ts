import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { fetchMessages, fetchConversations } from '$lib/api/dm';

export const load: PageServerLoad = async ({ locals, request, params }) => {
	if (!locals.user) throw redirect(303, '/');
	const cookies = request.headers.get('cookie') ?? '';
	const uuid = params.uuid;

	// Fetch initial messages and conversation list to find the other user info.
	const [messagesRes, conversations] = await Promise.all([
		fetchMessages(uuid, { limit: 50 }, cookies),
		fetchConversations(cookies).catch(() => [])
	]);

	// Find this conversation in the list to get the other user's info.
	const conv = conversations.find((c) => c.uuid === uuid);

	return {
		user: locals.user,
		conversationUuid: uuid,
		messages: messagesRes.data,
		cursor: messagesRes.cursor,
		hasMore: messagesRes.has_more,
		otherUser: conv?.other_user ?? null
	};
};
