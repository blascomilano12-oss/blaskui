<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { onMount, getContext } from 'svelte';
	import { getOpenAIConfig, updateOpenAIConfig, verifyOpenAIConnection } from '$lib/apis/openai';
	import { getModels as _getModels, getBackendConfig } from '$lib/apis';
	import { config, models, user } from '$lib/stores';
	import Switch from '$lib/components/common/Switch.svelte';
	import Spinner from '$lib/components/common/Spinner.svelte';

	const i18n: any = getContext('i18n');

	const OPENROUTER_DEFAULT = 'https://openrouter.ai/api/v1';
	const OMNIROUTE_DEFAULTS = ['https://api.omniroute.ai/v1', 'http://localhost:8081/v1', 'http://localhost:3000/v1'];

	let ENABLE_OPENAI_API: boolean | null = null;
	let OPENAI_API_BASE_URLS: string[] = [];
	let OPENAI_API_KEYS: string[] = [];
	let OPENAI_API_CONFIGS: any = {};

	let loading = true;
	let saving = false;

	// per-provider state
	let openRouterUrl = OPENROUTER_DEFAULT;
	let openRouterKey = '';
	let openRouterEnabled = true;
	let openRouterVerifying = false;
	let openRouterVerified: boolean | null = null;
	let openRouterModels = 0;

	let omniRouteUrl = OMNIROUTE_DEFAULTS[0];
	let omniRouteKey = '';
	let omniRouteEnabled = true;
	let omniRouteVerifying = false;
	let omniRouteVerified: boolean | null = null;
	let omniRouteModels = 0;

	const getModels = async () => {
		const m = await _getModels(localStorage.token, null, false, true);
		return m;
	};

	const findIdx = (urls: string[], matcher: (u: string) => boolean) => urls.findIndex(matcher);

	const syncFromConfig = () => {
		// OpenRouter
		let idx = findIdx(OPENAI_API_BASE_URLS, (u) => u.includes('openrouter.ai'));
		if (idx !== -1) {
			openRouterUrl = OPENAI_API_BASE_URLS[idx];
			openRouterKey = OPENAI_API_KEYS[idx] ?? '';
			openRouterEnabled = OPENAI_API_CONFIGS[idx]?.enable ?? true;
		} else {
			openRouterUrl = OPENROUTER_DEFAULT;
			openRouterKey = '';
			openRouterEnabled = false;
		}
		// OmniRoute - match any of defaults or contains omniroute
		idx = findIdx(OPENAI_API_BASE_URLS, (u) => u.includes('omniroute') || OMNIROUTE_DEFAULTS.includes(u));
		if (idx !== -1) {
			omniRouteUrl = OPENAI_API_BASE_URLS[idx];
			omniRouteKey = OPENAI_API_KEYS[idx] ?? '';
			omniRouteEnabled = OPENAI_API_CONFIGS[idx]?.enable ?? true;
		} else {
			omniRouteUrl = OMNIROUTE_DEFAULTS[0];
			omniRouteKey = '';
			omniRouteEnabled = false;
		}
	};

	const buildAndSave = async (
		provider: 'openrouter' | 'omniroute',
		url: string,
		key: string,
		enabled: boolean
	) => {
		url = url.replace(/\/$/, '');
		if (!url) {
			toast.error('URL mancante');
			return;
		}
		saving = true;
		try {
			// reload fresh to avoid race
			const cfg = await getOpenAIConfig(localStorage.token);
			let urls: string[] = cfg.OPENAI_API_BASE_URLS ?? [];
			let keys: string[] = cfg.OPENAI_API_KEYS ?? [];
			let configs: any = cfg.OPENAI_API_CONFIGS ?? {};
			let enable: boolean = cfg.ENABLE_OPENAI_API ?? true;

			// find idx for this provider
			const isOmni = provider === 'omniroute';
			const matcher = isOmni
				? (u: string) => u.includes('omniroute') || OMNIROUTE_DEFAULTS.includes(u)
				: (u: string) => u.includes('openrouter.ai');

			let idx = findIdx(urls, matcher);

			// normalize keys length before mutation
			if (keys.length !== urls.length) {
				if (keys.length > urls.length) keys = keys.slice(0, urls.length);
				else while (keys.length < urls.length) keys.push('');
			}

			if (idx === -1) {
				// add new
				urls = [...urls, url];
				keys = [...keys, key];
				const newIdx = urls.length - 1;
				configs[newIdx] = {
					enable: enabled,
					prefix_id: isOmni ? 'omniroute' : 'openrouter',
					tags: [{ name: isOmni ? 'OmniRoute' : 'OpenRouter' }],
					model_ids: []
				};
			} else {
				urls[idx] = url;
				keys[idx] = key;
				configs[idx] = {
					...(configs[idx] ?? {}),
					enable: enabled,
					prefix_id: isOmni ? 'omniroute' : 'openrouter'
				};
				// legacy url-key cleanup
				if (configs[urls[idx]] && idx.toString() !== urls[idx]) delete configs[urls[idx]];
			}

			enable = true;
			const res = await updateOpenAIConfig(localStorage.token, {
				ENABLE_OPENAI_API: enable,
				OPENAI_API_BASE_URLS: urls,
				OPENAI_API_KEYS: keys,
				OPENAI_API_CONFIGS: configs
			});
			if (res) {
				ENABLE_OPENAI_API = res.ENABLE_OPENAI_API;
				OPENAI_API_BASE_URLS = res.OPENAI_API_BASE_URLS;
				OPENAI_API_KEYS = res.OPENAI_API_KEYS;
				OPENAI_API_CONFIGS = res.OPENAI_API_CONFIGS;
				syncFromConfig();
				toast.success(`${provider === 'omniroute' ? 'OmniRoute' : 'OpenRouter'} salvato`);
				await models.set(await getModels());
				await config.set(await getBackendConfig());
			}
		} catch (e) {
			toast.error(`${e}`);
		} finally {
			saving = false;
		}
	};

	const verify = async (provider: 'openrouter' | 'omniroute') => {
		const url = provider === 'omniroute' ? omniRouteUrl.replace(/\/$/, '') : openRouterUrl.replace(/\/$/, '');
		const key = provider === 'omniroute' ? omniRouteKey : openRouterKey;
		if (!key) {
			toast.error('Inserisci prima la API key');
			return;
		}
		const setterVerifying = provider === 'omniroute' ? (v: boolean) => (omniRouteVerifying = v) : (v: boolean) => (openRouterVerifying = v);
		const setterVerified = provider === 'omniroute' ? (v: boolean) => (omniRouteVerified = v) : (v: boolean) => (openRouterVerified = v);
		const setterModels = provider === 'omniroute' ? (n: number) => (omniRouteModels = n) : (n: number) => (openRouterModels = n);

		setterVerifying(true);
		try {
			const res = await verifyOpenAIConnection(localStorage.token, {
				url,
				key,
				config: { enable: true, prefix_id: provider === 'omniroute' ? 'omniroute' : 'openrouter' }
			});
			// verify returns {data: [...]} or similar
			const count = res?.data?.length ?? res?.models?.length ?? 0;
			setterModels(count);
			setterVerified(true);
			toast.success(`Verificato: ${count} modelli trovati`);
		} catch (e) {
			setterVerified(false);
			toast.error(`Verifica fallita: ${e}`);
		} finally {
			setterVerifying(false);
		}
	};

	const removeProvider = async (provider: 'openrouter' | 'omniroute') => {
		const matcher =
			provider === 'omniroute'
				? (u: string) => u.includes('omniroute') || OMNIROUTE_DEFAULTS.includes(u)
				: (u: string) => u.includes('openrouter.ai');
		const cfg = await getOpenAIConfig(localStorage.token);
		let urls: string[] = cfg.OPENAI_API_BASE_URLS ?? [];
		let keys: string[] = cfg.OPENAI_API_KEYS ?? [];
		let configs: any = cfg.OPENAI_API_CONFIGS ?? {};
		const idx = findIdx(urls, matcher);
		if (idx === -1) {
			toast.error('Nessuna connessione da rimuovere');
			return;
		}
		urls = urls.filter((_, i) => i !== idx);
		keys = keys.filter((_, i) => i !== idx);
		let newConfigs: any = {};
		urls.forEach((u, newIdx) => {
			const oldIdx = newIdx < idx ? newIdx : newIdx + 1;
			newConfigs[newIdx] = configs[oldIdx] ?? configs[urls[newIdx]] ?? {};
		});
		const res = await updateOpenAIConfig(localStorage.token, {
			ENABLE_OPENAI_API: cfg.ENABLE_OPENAI_API,
			OPENAI_API_BASE_URLS: urls,
			OPENAI_API_KEYS: keys,
			OPENAI_API_CONFIGS: newConfigs
		});
		if (res) {
			OPENAI_API_BASE_URLS = res.OPENAI_API_BASE_URLS;
			OPENAI_API_KEYS = res.OPENAI_API_KEYS;
			OPENAI_API_CONFIGS = res.OPENAI_API_CONFIGS;
			syncFromConfig();
			toast.success('Rimosso');
			await models.set(await getModels());
		}
	};

	onMount(async () => {
		if ($user?.role !== 'admin') {
			loading = false;
			return;
		}
		try {
			const cfg = await getOpenAIConfig(localStorage.token);
			ENABLE_OPENAI_API = cfg.ENABLE_OPENAI_API ?? true;
			OPENAI_API_BASE_URLS = cfg.OPENAI_API_BASE_URLS ?? [];
			OPENAI_API_KEYS = cfg.OPENAI_API_KEYS ?? [];
			OPENAI_API_CONFIGS = cfg.OPENAI_API_CONFIGS ?? {};
			syncFromConfig();
		} catch (e) {
			toast.error(`${e}`);
		} finally {
			loading = false;
		}
	});
</script>

{#if $user?.role !== 'admin'}
	<div class="py-8 text-center text-sm text-gray-500">Solo admin può configurare i provider AI.</div>
{:else if loading}
	<div class="flex h-32 items-center justify-center"><Spinner className="size-5" /></div>
{:else}
	<form class="flex h-full flex-col text-sm" on:submit|preventDefault>
		<div class="flex-1 space-y-6 overflow-y-auto pr-1.5">
			<div>
				<h2 class="text-sm font-medium text-gray-900 dark:text-white">AI Providers</h2>
				<p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
					Collega OpenRouter e/o OmniRoute come provider OpenAI-compatibili. Le connessioni usano lo stesso backend di <span class="font-mono">/openai</span> — niente duplicazione, solo preset veloci.
				</p>
			</div>

			<!-- OpenRouter -->
			<div class="rounded-xl border border-gray-200 dark:border-gray-800 p-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<div class="flex size-7 items-center justify-center rounded-lg bg-[#7aa2ff]/10 text-[#7aa2ff] text-xs font-bold">OR</div>
						<div>
							<div class="text-sm font-medium">OpenRouter</div>
							<div class="text-xs text-gray-500">https://openrouter.ai/api/v1 — 300+ modelli</div>
						</div>
					</div>
					<Switch bind:state={openRouterEnabled} />
				</div>

				<div class="mt-3 grid gap-2">
					<label class="text-xs font-medium">Base URL</label>
					<input
						class="w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-xs"
						bind:value={openRouterUrl}
						placeholder={OPENROUTER_DEFAULT}
					/>
					<label class="text-xs font-medium">API Key</label>
					<input
						type="password"
						class="w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-xs"
						bind:value={openRouterKey}
						placeholder="sk-or-v1-..."
					/>
					<div class="flex gap-2">
						<button
							type="button"
							class="rounded-full border border-gray-200 dark:border-gray-700 px-3 py-1.5 text-xs hover:bg-gray-50 dark:hover:bg-gray-800 disabled:opacity-50"
							disabled={openRouterVerifying || !openRouterKey}
							on:click={() => verify('openrouter')}
						>
							{#if openRouterVerifying}<Spinner className="size-3 inline mr-1" />{/if}Verifica
						</button>
						<button
							type="button"
							class="rounded-full bg-black text-white dark:bg-white dark:text-black px-4 py-1.5 text-xs hover:opacity-90 disabled:opacity-50"
							disabled={saving}
							on:click={() => buildAndSave('openrouter', openRouterUrl, openRouterKey, openRouterEnabled)}
						>
							Salva
						</button>
						<button
							type="button"
							class="rounded-full px-3 py-1.5 text-xs text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
							on:click={() => removeProvider('openrouter')}
						>
							Rimuovi
						</button>
						{#if openRouterVerified !== null}
							<span class="ml-auto text-xs {openRouterVerified ? 'text-green-600' : 'text-red-600'}">
								{openRouterVerified ? `✓ ${openRouterModels} modelli` : '✗ verifica fallita'}
							</span>
						{/if}
					</div>
				</div>
			</div>

			<!-- OmniRoute -->
			<div class="rounded-xl border border-gray-200 dark:border-gray-800 p-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<div class="flex size-7 items-center justify-center rounded-lg bg-[#3d5cff]/10 text-[#3d5cff] text-xs font-bold">OM</div>
						<div>
							<div class="text-sm font-medium">OmniRoute</div>
							<div class="text-xs text-gray-500">Gateway locale — 359 provider via unico endpoint</div>
						</div>
					</div>
					<Switch bind:state={omniRouteEnabled} />
				</div>

				<div class="mt-3 grid gap-2">
					<label class="text-xs font-medium">Base URL</label>
					<input
						class="w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-xs"
						bind:value={omniRouteUrl}
						placeholder="https://api.omniroute.ai/v1 oppure http://localhost:8081/v1"
					/>
					<div class="text-[11px] text-gray-400">Preset: {OMNIROUTE_DEFAULTS.join(' · ')}</div>
					<label class="text-xs font-medium">API Key</label>
					<input
						type="password"
						class="w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-xs"
						bind:value={omniRouteKey}
						placeholder="sk-omni-... (lascia vuoto se gateway locale senza auth)"
					/>
					<div class="flex gap-2">
						<button
							type="button"
							class="rounded-full border border-gray-200 dark:border-gray-700 px-3 py-1.5 text-xs hover:bg-gray-50 dark:hover:bg-gray-800 disabled:opacity-50"
							disabled={omniRouteVerifying || (!omniRouteKey && !omniRouteUrl.includes('localhost'))}
							on:click={() => verify('omniroute')}
						>
							{#if omniRouteVerifying}<Spinner className="size-3 inline mr-1" />{/if}Verifica
						</button>
						<button
							type="button"
							class="rounded-full bg-black text-white dark:bg-white dark:text-black px-4 py-1.5 text-xs hover:opacity-90 disabled:opacity-50"
							disabled={saving}
							on:click={() => buildAndSave('omniroute', omniRouteUrl, omniRouteKey, omniRouteEnabled)}
						>
							Salva
						</button>
						<button
							type="button"
							class="rounded-full px-3 py-1.5 text-xs text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
							on:click={() => removeProvider('omniroute')}
						>
							Rimuovi
						</button>
						{#if omniRouteVerified !== null}
							<span class="ml-auto text-xs {omniRouteVerified ? 'text-green-600' : 'text-red-600'}">
								{omniRouteVerified ? `✓ ${omniRouteModels} modelli` : '✗ verifica fallita'}
							</span>
						{/if}
					</div>
				</div>
			</div>

			<div class="rounded-lg bg-gray-50 dark:bg-gray-800/50 p-3 text-[11px] text-gray-500 dark:text-gray-400">
				Tip: puoi anche gestire tutte le connessioni OpenAI generiche da <span class="font-mono">Admin → Connections</span>. Questa sezione è solo un preset veloce per i due provider più usati in BlaskUI.
			</div>
		</div>
	</form>
{/if}
