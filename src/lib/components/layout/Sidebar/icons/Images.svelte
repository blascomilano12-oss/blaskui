<script lang="ts">
	export let className = 'size-4';
	export let strokeWidth = '1.5';
</script>

<svg
	class={className}
	aria-hidden="true"
	xmlns="http://www.w3.org/2000/svg"
	viewBox="0 0 24 24"
	stroke-width={strokeWidth}
	fill="none"
>
	<rect x="3" y="3" width="18" height="18" rx="2" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" />
	<circle cx="9" cy="9" r="2" stroke="currentColor" />
	<path d="M21 15L16 10L5 21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" />
</svg>
