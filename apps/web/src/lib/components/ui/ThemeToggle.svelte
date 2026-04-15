<script lang="ts">
	import { theme } from '$lib/stores/theme';
</script>

<button
	type="button"
	onclick={theme.toggle}
	aria-label={$theme === 'dark' ? 'Zet licht modus aan' : 'Zet donker modus aan'}
	title={$theme === 'dark' ? 'Licht' : 'Donker'}
	class="rounded-full p-2 text-lg leading-none hover:bg-border"
>
	{$theme === 'dark' ? '☀️' : '🌙'}
</button>
