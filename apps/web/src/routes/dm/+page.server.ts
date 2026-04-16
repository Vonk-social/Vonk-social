import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { fetchSnapInbox, fetchSentSnaps } from '$lib/api/snaps';
import { fetchConversations } from '$lib/api/dm';

export const load: PageServerLoad = async ({ locals, request }) => {
	if (!locals.user) throw redirect(303, '/');
	const cookies = request.headers.get('cookie') ?? '';
	const [conversations, inbox, sent] = await Promise.all([
		fetchConversations(cookies).catch(() => []),
		fetchSnapInbox(cookies).catch(() => []),
		fetchSentSnaps(cookies).catch(() => [])
	]);
	return { user: locals.user, conversations, inbox, sent };
};
