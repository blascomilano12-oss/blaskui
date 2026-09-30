<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { gsap } from 'gsap';

	// MagneticCursor Svelte: replica il comportamento React con gsap.
	// Elementi con data-magnetic si attirano al cursore con elastic.out.
	// Il cursore si espande sull'elemento e riprende forma all'uscita.

	let cursorEl: HTMLDivElement;
	let cleanupFns: (() => void)[] = [];

	const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

	onMount(() => {
		if (!window.matchMedia?.('(hover: hover) and (pointer: fine)').matches) return;

		const prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		const detachDuration = prefersReduced ? 0.1 : 0.35;
		const lerpAmount = prefersReduced ? 1 : 0.1;
		const magneticFactor = 0.2;
		const hoverPadding = 12;
		const cursorSize = 24;

		gsap.set(cursorEl, { xPercent: -50, yPercent: -50 });

		const pos = { current: { x: -100, y: -100 }, target: { x: -100, y: -100 }, previous: { x: -100, y: -100 } };
		let isHovered = false;
		let isDetaching = false;

		const update = () => {
			if (isHovered) return;
			pos.current.x = lerp(pos.current.x, pos.target.x, lerpAmount);
			pos.current.y = lerp(pos.current.y, pos.target.y, lerpAmount);
			const dx = pos.current.x - pos.previous.x;
			const dy = pos.current.y - pos.previous.y;
			pos.previous.x = pos.current.x;
			pos.previous.y = pos.current.y;

			if (isDetaching) {
				gsap.set(cursorEl, { x: pos.current.x, y: pos.current.y, scaleX: 1, scaleY: 1, rotate: 0, overwrite: 'auto' });
			} else {
				const speed = Math.sqrt(dx * dx + dy * dy) * 0.02;
				gsap.set(cursorEl, {
					x: pos.current.x,
					y: pos.current.y,
					rotate: Math.atan2(dy, dx) * (180 / Math.PI),
					scaleX: 1 + Math.min(speed, 1),
					scaleY: 1 - Math.min(speed, 0.3),
					overwrite: 'auto'
				});
			}
		};

		const onPointerMove = (e: PointerEvent) => {
			pos.target.x = e.clientX;
			pos.target.y = e.clientY;
			const inViewport = e.clientX >= 0 && e.clientX <= window.innerWidth && e.clientY >= 0 && e.clientY <= window.innerHeight;
			gsap.to(cursorEl, { opacity: inViewport ? 1 : 0, duration: 0.2, overwrite: 'auto' });

			const target = e.target as HTMLElement;
			const isText = ['P', 'SPAN', 'H1', 'H2', 'H3', 'H4', 'H5', 'H6'].includes(target.tagName) || window.getComputedStyle(target).cursor === 'text';
			if (isText && !isHovered && !isDetaching) {
				gsap.to(cursorEl, { scaleX: 0.5, scaleY: 1.5, duration: 0.3, overwrite: 'auto' });
			}
		};

		const initPosition = (e: MouseEvent) => {
			pos.current.x = e.clientX;
			pos.current.y = e.clientY;
			pos.target.x = e.clientX;
			pos.target.y = e.clientY;
			pos.previous.x = e.clientX;
			pos.previous.y = e.clientY;
			gsap.set(cursorEl, { x: e.clientX, y: e.clientY, opacity: 1 });
		};

		gsap.ticker.add(update);
		window.addEventListener('pointermove', onPointerMove);
		window.addEventListener('pointermove', initPosition, { once: true });
		document.addEventListener('mouseleave', () => gsap.to(cursorEl, { opacity: 0, duration: 0.3 }));
		document.addEventListener('mouseenter', () => gsap.to(cursorEl, { opacity: 1, duration: 0.3 }));

		// Elementi magnetici
		const magneticEls = document.querySelectorAll<HTMLElement>('[data-magnetic]');
		magneticEls.forEach((el) => {
			const xTo = gsap.quickTo(el, 'x', { duration: 1, ease: 'elastic.out(1, 0.3)' });
			const yTo = gsap.quickTo(el, 'y', { duration: 1, ease: 'elastic.out(1, 0.3)' });

			const onEnter = () => {
				isHovered = true;
				isDetaching = false;
				const bounds = el.getBoundingClientRect();
				const computed = window.getComputedStyle(el);
				const color = el.getAttribute('data-magnetic-color') || '#fff';
				const pad = hoverPadding * (1 + magneticFactor);
				const cx = bounds.left + bounds.width / 2;
				const cy = bounds.top + bounds.height / 2;

				gsap.killTweensOf(cursorEl);
				gsap.to(cursorEl, {
					x: cx, y: cy,
					width: bounds.width + pad * 2,
					height: bounds.height + pad * 2,
					borderRadius: computed.borderRadius,
					backgroundColor: color,
					scaleX: 1, scaleY: 1, rotate: 0,
					duration: 0.3, ease: 'power3.out', overwrite: 'all'
				});
			};

			const onLeave = () => {
				const cx = gsap.getProperty(cursorEl, 'x') as number;
				const cy = gsap.getProperty(cursorEl, 'y') as number;
				pos.current.x = cx; pos.current.y = cy;
				pos.previous.x = cx; pos.previous.y = cy;
				isHovered = false;
				isDetaching = true;

				gsap.killTweensOf(cursorEl);
				gsap.to(cursorEl, {
					width: cursorSize, height: cursorSize,
					borderRadius: '50%',
					backgroundColor: '#fff',
					scaleX: 1, scaleY: 1,
					duration: detachDuration, ease: 'power3.out', overwrite: 'all',
					onComplete: () => { isDetaching = false; }
				});
			};

			const onMove = (e: PointerEvent) => {
				const { height, width, left, top } = el.getBoundingClientRect();
				xTo((e.clientX - (left + width / 2)) * magneticFactor);
				yTo((e.clientY - (top + height / 2)) * magneticFactor);
			};

			const onOut = () => { xTo(0); yTo(0); };

			el.addEventListener('pointerenter', onEnter);
			el.addEventListener('pointerleave', onLeave);
			el.addEventListener('pointermove', onMove);
			el.addEventListener('pointerout', onOut);

			cleanupFns.push(() => {
				el.removeEventListener('pointerenter', onEnter);
				el.removeEventListener('pointerleave', onLeave);
				el.removeEventListener('pointermove', onMove);
				el.removeEventListener('pointerout', onOut);
			});
		});

		return () => {
			gsap.ticker.remove(update);
			window.removeEventListener('pointermove', onPointerMove);
			document.removeEventListener('mouseleave', () => gsap.to(cursorEl, { opacity: 0, duration: 0.3 }));
			document.removeEventListener('mouseenter', () => gsap.to(cursorEl, { opacity: 1, duration: 0.3 }));
			cleanupFns.forEach((fn) => fn());
		};
	});

	onDestroy(() => {
		cleanupFns.forEach((fn) => fn());
	});
</script>

<div bind:this={cursorEl} class="magnetic-cursor" aria-hidden="true">
	<slot />
</div>

<style>
	.magnetic-cursor {
		position: fixed;
		top: 0;
		left: 0;
		z-index: 100002;
		pointer-events: none;
		will-change: transform, width, height, border-radius;
		background-color: #fff;
		mix-blend-mode: exclusion;
		width: 24px;
		height: 24px;
		border-radius: 50%;
		backdrop-filter: contrast(1.5);
		-webkit-backdrop-filter: contrast(1.5);
	}
</style>
