<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { gsap } from 'gsap';

	import { user } from '$lib/stores';

	// BlaskUI (Fase 2.3): headline + inviti rotanti personalizzati col nome utente.
	const displayName = $user?.name?.split(' ')[0] || 'creativo';

	const phrases = [
		`Pronto a creare qualcosa di grande, ${displayName}?`,
		`Dai vita a una nuova idea, ${displayName}.`,
		`Nuovo progetto? Si parte da qui, ${displayName}.`,
		`Oltre lo spazio, un prompt alla volta.`
	];

	let headEl: HTMLElement | null = null;
	let subEl: HTMLElement | null = null;
	let ctx: gsap.Context | null = null;

	onMount(() => {
		ctx = gsap.context(() => {
			gsap.from(headEl, { y: 28, opacity: 0, duration: 0.9, ease: 'power3.out' });
			gsap.to(headEl, {
				y: -6,
				duration: 2.8,
				ease: 'sine.inOut',
				repeat: -1,
				yoyo: true,
				delay: 1
			});

			const tl = gsap.timeline({ repeat: -1 });
			phrases.forEach((p) => {
				tl.call(() => {
					if (subEl) subEl.textContent = p;
				})
					.fromTo(
						subEl,
						{ y: 16, opacity: 0 },
						{ y: 0, opacity: 1, duration: 0.55, ease: 'power3.out' }
					)
					.to(subEl, { y: -12, opacity: 0, duration: 0.4, ease: 'power2.in' }, '+=3.4');
			});
		});
	});

	onDestroy(() => {
		ctx?.revert();
	});
</script>

<div class="flex w-full flex-col items-center justify-center px-5 text-center">
	<h1
		bind:this={headEl}
		class="bg-gradient-to-b from-white via-gray-100 to-gray-400 bg-clip-text text-4xl font-semibold tracking-tight text-transparent drop-shadow-[0_2px_18px_rgba(0,0,0,0.55)] @sm:text-5xl"
	>
		Ciao {displayName}.
	</h1>
	<p
		bind:this={subEl}
		class="mt-3 min-h-7 text-base font-normal text-gray-200 drop-shadow-[0_1px_10px_rgba(0,0,0,0.6)] @sm:text-lg"
		aria-live="polite"
	></p>
</div>
