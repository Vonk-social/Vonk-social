import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = ({ locals }) => {
	if (!locals.user) throw redirect(303, '/');
	if (locals.user.needs_onboarding) throw redirect(303, '/onboarding/username');
	return { user: locals.user };
};
