<script lang="ts">
	import { page } from '$app/stores';
</script>

<main class="mx-auto flex min-h-screen max-w-lg flex-col items-center justify-center px-6 py-16">
	<div class="vonk-card w-full text-center">
		<p class="text-sm font-semibold uppercase tracking-wider text-muted">
			Vonk — er ging iets mis
		</p>
		<h1 class="mt-2 font-display text-4xl font-extrabold text-terracotta">
			{$page.status}
		</h1>
		<p class="mt-4 text-muted">
			{$page.error?.message ?? 'Onbekende fout.'}
		</p>
		<div class="mt-8 flex justify-center gap-3">
			<a
				href="/"
				class="rounded-[var(--radius-button)] border border-border bg-white px-5 py-2.5 text-sm font-semibold text-ink hover:bg-border/40"
			>
				Terug naar start
			</a>
			<a
				href="/login"
				class="rounded-[var(--radius-button)] bg-terracotta px-5 py-2.5 text-sm font-semibold text-white hover:bg-terracotta-dark"
			>
				Opnieuw aanmelden
			</a>
		</div>
	</div>
</main>
