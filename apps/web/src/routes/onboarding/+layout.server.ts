import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';

/**
 * Onboarding guard:
 *   • not logged in → back to `/`
 *   • already onboarded → straight to `/home`
 * Individual step pages decide which step to show based on `user` state.
 */
export const load: LayoutServerLoad = ({ locals, url }) => {
	if (!locals.user) throw redirect(303, '/login');
	if (!locals.user.needs_onboarding && url.pathname !== '/onboarding/friends') {
		throw redirect(303, '/home');
	}
	return { user: locals.user };
};
