<script lang="ts">
	import { onMount } from 'svelte';
	import { spring } from 'svelte/motion';
	import { user } from '$lib/stores';

	// Cursore BlaskUI: freccia con fisica a molla, inclinazione dalla velocità,
	// tag col nome utente che sparisce/riappare con doppio scuotimento.
	let enabled = false;
	let overText = false;
	let overMenu = false;
	let visible = false;
	let tagVisible = true;

	const pos = spring({ x: -100, y: -100 }, { stiffness: 300, damping: 25 });
	const tag = spring({ x: -100, y: -100 }, { stiffness: 180, damping: 22 });

	let lastX = -100;
	let lastY = -100;
	let lastT = 0;
	let vx = 0;
	let vy = 0;

	// shake detection: conta i cambi di direzione X entro una finestra temporale
	let lastDir = 0;
	let dirChanges: number[] = [];
	const SHAKE_WINDOW = 500; // ms per 2 scuotimenti
	const CHANGES_PER_SHAKE = 2; // ogni scuotimento = 2 cambi di direzione

	const onMove = (e: MouseEvent) => {
		visible = true;
		const now = performance.now();
		const dt = Math.max(1, now - lastT);
		vx = vx * 0.7 + ((e.clientX - lastX) / dt) * 1000 * 0.3;
		vy = vy * 0.7 + ((e.clientY - lastY) / dt) * 1000 * 0.3;
		lastX = e.clientX;
		lastY = e.clientY;
		lastT = now;
		pos.set({ x: e.clientX, y: e.clientY });
		tag.set({ x: e.clientX, y: e.clientY });

		// shake detection
		const dx = e.clientX - lastX;
		if (Math.abs(dx) > 2) {
			const dir = dx > 0 ? 1 : -1;
			if (dir !== lastDir && lastDir !== 0) {
				dirChanges.push(now);
				// mantieni solo i cambi recenti
				dirChanges = dirChanges.filter((t) => now - t < SHAKE_WINDOW);
				// 2 scuotimenti = 4 cambi di direzione
				if (dirChanges.length >= CHANGES_PER_SHAKE * 2) {
					tagVisible = !tagVisible;
					dirChanges = [];
				}
			}
			lastDir = dir;
		}

		const target = e.target as HTMLElement | null;
		overText = !!target?.closest?.('input, textarea, [contenteditable="true"]');
		overMenu = !!target?.closest?.(
			'[role="menu"], [role="listbox"], [role="dialog"], [role="menuitem"], .tippy-box, [data-tippy-root], dialog, [data-radix-popper-content-wrapper]'
		);
	};

	onMount(() => {
		if (!window.matchMedia?.('(hover: hover) and (pointer: fine)').matches) return;
		enabled = true;
		lastT = performance.now();
		document.documentElement.classList.add('blask-cursor');
		window.addEventListener('mousemove', onMove, { passive: true });
		document.documentElement.addEventListener('mouseleave', () => (visible = false));
		return () => {
			window.removeEventListener('mousemove', onMove);
			document.documentElement.classList.remove('blask-cursor');
		};
	});

	$: speed = Math.sqrt(vx * vx + vy * vy);
	$: tilt = Math.max(-45, Math.min(45, ((vx + vy) / 1000) * 30));
	$: squash = 1 - Math.min(speed / 2000, 0.1);
</script>

{#if enabled}
	<!-- freccia: sempre sopra tutto -->
	<div
		class="pointer-events-none fixed top-0 left-0 z-[100001] transition-opacity duration-150"
		style="opacity: {visible ? 1 : 0}; transform: translate({$pos.x}px, {$pos.y}px);"
		aria-hidden="true"
	>
		{#if overText}
			<div class="h-5 w-[2px] -translate-x-1/2 rounded bg-white mix-blend-difference"></div>
		{:else}
			<svg
				width="26"
				height="26"
				viewBox="0 0 40 40"
				style="transform: translate(-4.5%, -11%) rotate({tilt}deg) scale({squash}); transform-origin: 4.5% 11%;"
			>
				<path
					fill="#fff"
					stroke="#000"
					stroke-opacity="0.35"
					stroke-width="1.5"
					d="M1.8 4.4 7 36.2c.3 1.8 2.6 2.3 3.6.8l3.9-5.7c1.7-2.5 4.5-4.1 7.5-4.3l6.9-.5c1.8-.1 2.5-2.4 1.1-3.5L5 2.5c-1.4-1.1-3.5 0-3.3 1.9Z"
				/>
			</svg>
		{/if}
	</div>
	<!-- tag nome: sparisce/riappare con doppio scuotimento, si ritira sui menu -->
	{#if $user?.name}
		<div
			class="pointer-events-none fixed top-0 left-0 z-[100000] transition-all duration-300 ease-out"
			style="opacity: {visible && !overMenu && tagVisible ? 1 : 0}; transform: translate({$tag.x + 22}px, {$tag.y + 18}px) scale({visible && !overMenu && tagVisible ? 1 : 0.6});"
			aria-hidden="true"
		>
			<div
				class="rounded-md border border-white/15 bg-black/70 px-2 py-0.5 text-xs whitespace-nowrap text-white backdrop-blur"
			>
				{$user.name}
			</div>
		</div>
	{/if}
{/if}

<style>
	:global(html.blask-cursor),
	:global(html.blask-cursor *) {
		cursor: none !important;
	}
</style>
