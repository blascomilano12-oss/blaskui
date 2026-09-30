<script lang="ts">
	import { onMount } from 'svelte';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { WEBUI_NAME } from '$lib/stores';

	let appWindow: ReturnType<typeof getCurrentWindow> | null = null;
	let isMaximized = false;

	onMount(async () => {
		try {
			appWindow = getCurrentWindow();
			isMaximized = await appWindow.isMaximized();
		} catch {
			// non-Tauri (browser dev): ignora
		}
	});

	const toggleMaximize = async () => {
		if (!appWindow) return;
		try {
			await appWindow.toggleMaximize();
			isMaximized = await appWindow.isMaximized();
		} catch {
			// ignora
		}
	};
</script>

<div class="titlebar" data-tauri-drag-region>
	<div class="titlebar-left" data-tauri-drag-region>
		<img src="/assets/brand/blaskui-logo.svg" class="titlebar-logo" alt="" />
		<span class="titlebar-title">{$WEBUI_NAME}</span>
	</div>
	<div class="titlebar-controls">
		<button class="tb-btn" on:click={() => appWindow.minimize()} aria-label="Minimizza">
			<svg width="10" height="10" viewBox="0 0 10 10"><line x1="0" y1="5" x2="10" y2="5" stroke="currentColor" stroke-width="1.5"/></svg>
		</button>
		<button class="tb-btn" on:click={toggleMaximize} aria-label="Massimizza">
			{#if isMaximized}
				<svg width="10" height="10" viewBox="0 0 10 10"><rect x="0" y="0" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1.2"/><rect x="3" y="3" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1.2"/></svg>
			{:else}
				<svg width="10" height="10" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1.2"/></svg>
			{/if}
		</button>
		<button class="tb-btn tb-close" on:click={() => appWindow.close()} aria-label="Chiudi">
			<svg width="10" height="10" viewBox="0 0 10 10"><line x1="0" y1="0" x2="10" y2="10" stroke="currentColor" stroke-width="1.5"/><line x1="10" y1="0" x2="0" y2="10" stroke="currentColor" stroke-width="1.5"/></svg>
		</button>
	</div>
</div>

<style>
	.titlebar {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 12px;
		z-index: 9999;
		background: rgba(10, 12, 20, 0.6);
		backdrop-filter: blur(20px) saturate(1.5);
		-webkit-backdrop-filter: blur(20px) saturate(1.5);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		user-select: none;
	}
	.titlebar-left {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.titlebar-logo {
		width: 18px;
		height: 18px;
		filter: invert(1);
	}
	.titlebar-title {
		font-size: 13px;
		font-weight: 500;
		color: rgba(255, 255, 255, 0.85);
	}
	.titlebar-controls {
		display: flex;
		gap: 2px;
	}
	.tb-btn {
		width: 36px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		background: transparent;
		color: rgba(255, 255, 255, 0.7);
		border-radius: 6px;
		cursor: pointer;
		transition: background 0.15s;
	}
	.tb-btn:hover {
		background: rgba(255, 255, 255, 0.1);
	}
	.tb-close:hover {
		background: #e81123;
		color: #fff;
	}
</style>
