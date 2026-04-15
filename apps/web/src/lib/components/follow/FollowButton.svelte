<script lang="ts">
	import { untrack } from 'svelte';
	import { follow, unfollow, type FollowState } from '$lib/api/follows';
	import { toasts } from '$lib/stores/toasts';

	type Props = { username: string; initial: FollowState };
	let { username, initial }: Props = $props();

	let stateVal = $state<FollowState>(untrack(() => initial));
	let pending = $state(false);

	const label = $derived.by(() => {
		switch (stateVal) {
			case 'active':
				return 'Volgend';
			case 'pending':
				return 'Aangevraagd';
			case 'self':
				return 'Jij';
			default:
				return 'Volgen';
		}
	});

	async function toggle() {
		if (pending || stateVal === 'self') return;
		pending = true;
		try {
			if (stateVal === 'none') {
				const next = await follow(username);
				stateVal = (next as FollowState) ?? 'active';
			} else {
				// active or pending → unfollow/cancel
				await unfollow(username);
				stateVal = 'none';
			}
		} catch (e) {
			toasts.push('error', (e as Error).message);
		} finally {
			pending = false;
		}
	}
</script>

<button
	type="button"
	onclick={toggle}
	disabled={pending || stateVal === 'self'}
	class="rounded-full px-5 py-2 text-sm font-semibold transition-colors disabled:opacity-60"
	class:bg-terracotta={stateVal === 'none'}
	class:text-white={stateVal === 'none'}
	class:bg-white={stateVal !== 'none'}
	class:text-ink={stateVal !== 'none'}
	class:border={stateVal !== 'none'}
	class:border-border={stateVal !== 'none'}
>
	{label}
</button>
