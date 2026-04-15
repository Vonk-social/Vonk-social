<script lang="ts">
	type Props = {
		/** Pixel size (applied to both width and height). */
		size?: number;
		/** `gradient` = brand full-colour; `mono` = solid `currentColor`-compatible. */
		variant?: 'gradient' | 'mono';
		/** Forwarded to the <img>/<svg>. Use this for the label shown to screen readers. */
		alt?: string;
	};

	let { size = 48, variant = 'gradient', alt = 'Vonk' }: Props = $props();
</script>

{#if variant === 'gradient'}
	<!--
		The full-colour mark has a three-stop gradient that cannot be themed
		via currentColor, so we reference the static SVG asset directly. The
		alt text is what screen readers announce.
	-->
	<img
		src="/icons/vonk-icon.svg"
		width={size}
		height={size}
		{alt}
		class="inline-block select-none"
		draggable="false"
	/>
{:else}
	<!--
		Mono variant inlined so callers can override the fill via CSS
		(`color: var(--color-terracotta)` on a parent). The path data matches
		the one in static/icons/vonk-icon-mono.svg.
	-->
	<svg
		xmlns="http://www.w3.org/2000/svg"
		viewBox="0 0 468 441"
		width={size}
		height={size}
		role="img"
		aria-label={alt}
		class="inline-block select-none"
	>
		<path
			d="M 315 8 L 299 41 L 290 53 L 279 62 L 272 64 L 217 62 L 154 63 L 131 65 L 98 71 L 74 79 L 53 90 L 36 104 L 23 120 L 16 132 L 6 158 L 0 193 L 0 235 L 5 268 L 15 300 L 32 331 L 49 349 L 66 359 L 73 361 L 85 359 L 92 352 L 94 347 L 111 323 L 136 295 L 169 264 L 197 242 L 243 213 L 247 216 L 254 226 L 226 250 L 197 281 L 167 321 L 153 343 L 126 394 L 116 418 L 118 428 L 124 432 L 136 430 L 155 413 L 172 401 L 184 395 L 202 389 L 218 386 L 299 386 L 331 384 L 364 379 L 389 372 L 411 362 L 426 352 L 444 333 L 453 317 L 460 299 L 465 276 L 467 260 L 467 222 L 461 178 L 448 138 L 438 119 L 422 99 L 422 110 L 419 117 L 414 124 L 401 136 L 368 155 L 354 147 L 348 141 L 362 132 L 373 121 L 380 110 L 381 103 L 370 93 L 368 89 L 368 82 L 374 74 L 382 70 L 357 65 L 339 65 L 328 55 L 323 45 L 317 22 Z M 368 238 L 373 252 L 379 257 L 390 261 L 378 265 L 372 271 L 368 284 L 363 270 L 358 265 L 346 261 L 359 256 L 363 252 Z M 129 221 L 133 227 L 138 229 L 133 231 L 129 237 L 126 232 L 122 229 Z M 112 136 L 119 150 L 133 156 L 122 160 L 116 166 L 113 175 L 111 176 L 105 163 L 97 158 L 89 156 L 95 155 L 103 151 L 108 145 L 110 138 Z M 166 105 L 168 111 L 176 116 L 171 118 L 166 125 L 164 121 L 157 116 L 164 111 Z M 313 19 L 315 27 L 316 69 L 320 105 L 323 122 L 330 140 L 339 150 L 352 157 L 386 170 L 412 178 L 373 187 L 340 197 L 327 203 L 319 210 L 312 221 L 303 242 L 279 312 L 269 235 L 264 217 L 257 205 L 248 196 L 230 187 L 189 172 L 161 164 L 164 162 L 184 160 L 227 151 L 241 147 L 257 139 L 268 127 L 283 98 L 298 62 Z"
			fill="currentColor"
			fill-rule="evenodd"
		/>
	</svg>
{/if}
