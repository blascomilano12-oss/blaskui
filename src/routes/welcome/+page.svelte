<script lang="ts">
	import { onMount, getContext, tick } from 'svelte';
	import type { Readable } from 'svelte/store';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { animate } from 'motion/mini';

	import { getBackendConfig } from '$lib/apis';
	import { userSignUp, getSessionUser } from '$lib/apis/auths';
	import { getOllamaModels } from '$lib/apis/ollama';
	import { config, user, WEBUI_NAME } from '$lib/stores';
	import { generateInitialsImage } from '$lib/utils';
	import { rememberBlaskAccount } from '$lib/utils/accounts';
	import { changeLanguage } from '$lib/i18n';
	import Spinner from '$lib/components/common/Spinner.svelte';

	const i18n =
		getContext<Readable<{ t: (key: string, opts?: object) => string; language: string }>>('i18n');

	let loaded = false;
	let step = 0;

	let name = '';
	let email = '';
	let password = '';
	let submitting = false;

	let ollamaOk = false;
	let installedModels: string[] = [];

	const RECOMMENDED = [
		{
			id: 'blask-spark',
			match: ['blask-spark', 'spark'],
			title: 'Spark X2.5 4B',
			meta: 'Q8_0 · ~4.1 GB · via Unsloth Studio',
			desc: 'The fast all-rounder: Italian chat, writing and everyday tasks.'
		},
		{
			id: 'blask-minicpm',
			match: ['blask-minicpm', 'minicpm'],
			title: 'MiniCPM 5 2B',
			meta: 'F16 · ~4.7 GB · via Ollama',
			desc: 'The little speedster: quick answers, great on modest GPUs.'
		}
	];

	const animateStep = async () => {
		await tick();
		animate(
			'[data-welcome-animate]',
			{ opacity: [0, 1], y: [18, 0] },
			// ponytail: niente stagger (non esportato da motion/mini), delay fisso
			{ duration: 0.45, delay: 0.05, easing: 'ease-out' }
		);
	};

	$: if (loaded) {
		animateStep();
	}

	const hasModel = (m: (typeof RECOMMENDED)[number]) =>
		installedModels.some((n) => m.match.some((k) => n.toLowerCase().includes(k)));

	const checkModels = async () => {
		try {
			const models = await getOllamaModels(localStorage.token ?? '');
			if (Array.isArray(models)) {
				ollamaOk = true;
				installedModels = models.map((m) => m?.name ?? m?.id ?? '').filter(Boolean);
			}
		} catch {
			ollamaOk = false;
		}
	};

	const signUpHandler = async () => {
		if (!name.trim() || !email.trim() || !password) {
			toast.error($i18n.t('Please fill in all fields.'));
			return;
		}
		submitting = true;
		try {
			const sessionUser = await userSignUp(
				name.trim(),
				email.trim(),
				password,
				generateInitialsImage(name.trim())
			).catch((error) => {
				toast.error(`${error}`);
				return null;
			});
			if (sessionUser) {
				if (sessionUser.token) {
					localStorage.token = sessionUser.token;
				}
				await user.set(sessionUser);
				await config.set(await getBackendConfig());
				// BlaskUI: ricorda il profilo su questa macchina per /accounts
				rememberBlaskAccount(sessionUser);
				toast.success($i18n.t('Account created. Welcome to BlaskUI!'));
				await checkModels();
				step = 2;
				await animateStep();
			}
		} finally {
			submitting = false;
		}
	};

	const pickLanguage = async (lang: string) => {
		changeLanguage(lang);
		step = 3;
		await checkModels();
		await animateStep();
	};

	onMount(async () => {
		if ($user) {
			goto('/');
			return;
		}
		let backendConfig = $config;
		try {
			backendConfig = await getBackendConfig();
			await config.set(backendConfig);
		} catch {
			goto('/auth');
			return;
		}
		if (!backendConfig?.onboarding) {
			// DB già configurato: niente wizard, vai al login
			goto('/auth');
			return;
		}
		loaded = true;
		await animateStep();
	});
</script>

<svelte:head>
	<title>{$i18n.t('Welcome to BlaskUI')}</title>
</svelte:head>

<div class="relative flex min-h-screen w-full items-center justify-center overflow-hidden text-white">
	<video
		class="absolute inset-0 h-full w-full object-cover"
		src="/assets/blask-bg.mp4"
		autoplay
		muted
		loop
		playsinline
		preload="auto"
		aria-hidden="true"
	></video>
	<div class="absolute inset-0 bg-black/55"></div>
	<div class="relative w-full max-w-2xl px-6 py-12">
		<div class="mb-8 text-center" data-welcome-animate>
			<img src="/assets/brand/blaskui-logo-animated.svg" class="mx-auto h-9 w-auto" alt="BlaskUI" />
			<div class="mt-3 flex items-center justify-center gap-2">
				{#each [0, 1, 2, 3, 4] as i}
					<div
						class="h-1.5 rounded-full transition-all duration-300 {i === step
							? 'w-8 bg-[#7aa2ff]'
							: i < step
								? 'w-4 bg-[#7aa2ff]/50'
								: 'w-4 bg-white/15'}"
					></div>
				{/each}
			</div>
		</div>

		{#if !loaded}
			<div class="flex justify-center py-16"><Spinner className="size-8" /></div>
		{:else if step === 0}
			<div class="text-center">
				<h1 class="text-3xl font-light tracking-tight lg:text-5xl" data-welcome-animate>
					{$i18n.t('Welcome to BlaskUI')}
				</h1>
				<p
					class="mx-auto mt-6 max-w-xl text-sm leading-relaxed font-light text-white/60 lg:text-base"
					data-welcome-animate
				>
					{$i18n.t(
						'Your personal AI, running on your own machine. Your chats stay here: fast, private, no cloud required.'
					)}
				</p>
				<div class="mx-auto mt-8 grid max-w-xl grid-cols-1 gap-3 text-left sm:grid-cols-3">
					{#each [$i18n.t('Private by design'), $i18n.t('Italian first'), $i18n.t('Free models included')] as badge}
						<div
							class="rounded-xl border border-white/10 bg-white/5 px-4 py-3 text-xs text-white/70"
							data-welcome-animate
						>
							{badge}
						</div>
					{/each}
				</div>
				<button
					class="mt-10 bg-white px-10 py-3 text-sm text-black transition hover:bg-white/90"
					on:click={() => {
						step = 1;
					}}
					data-welcome-animate
				>
					{$i18n.t('Start setup')}
				</button>
			</div>
		{:else if step === 1}
			<div class="mx-auto max-w-md">
				<h2 class="text-2xl font-light tracking-tight" data-welcome-animate>
					{$i18n.t('Create your admin account')}
				</h2>
				<p class="mt-2 text-sm font-light text-white/60" data-welcome-animate>
					{$i18n.t('The first account becomes the administrator. Only on this first launch.')}
				</p>
				<form
					class="mt-8 flex flex-col gap-4"
					on:submit|preventDefault={signUpHandler}
					data-welcome-animate
				>
					<label class="flex flex-col gap-1.5 text-left text-xs text-white/60">
						{$i18n.t('Name')}
						<input
							class="rounded-lg border border-white/10 bg-white/5 px-4 py-2.5 text-sm text-white outline-none placeholder:text-white/30 focus:border-[#7aa2ff]/60"
							placeholder={$i18n.t('Your name')}
							bind:value={name}
							autocomplete="name"
						/>
					</label>
					<label class="flex flex-col gap-1.5 text-left text-xs text-white/60">
						{$i18n.t('Email')}
						<input
							type="email"
							class="rounded-lg border border-white/10 bg-white/5 px-4 py-2.5 text-sm text-white outline-none placeholder:text-white/30 focus:border-[#7aa2ff]/60"
							placeholder="you@example.com"
							bind:value={email}
							autocomplete="email"
						/>
					</label>
					<label class="flex flex-col gap-1.5 text-left text-xs text-white/60">
						{$i18n.t('Password')}
						<input
							type="password"
							class="rounded-lg border border-white/10 bg-white/5 px-4 py-2.5 text-sm text-white outline-none placeholder:text-white/30 focus:border-[#7aa2ff]/60"
							placeholder="••••••••"
							bind:value={password}
							autocomplete="new-password"
						/>
					</label>
					<button
						type="submit"
						disabled={submitting}
						class="mt-2 flex items-center justify-center gap-2 bg-white px-8 py-3 text-sm text-black transition hover:bg-white/90 disabled:opacity-60"
					>
						{#if submitting}<Spinner className="size-4" />{/if}
						{$i18n.t('Create account and continue')}
					</button>
				</form>
			</div>
		{:else if step === 2}
			<div class="mx-auto max-w-md text-center">
				<h2 class="text-2xl font-light tracking-tight" data-welcome-animate>
					{$i18n.t('Choose your language')}
				</h2>
				<p class="mt-2 text-sm font-light text-white/60" data-welcome-animate>
					{$i18n.t('You can change it anytime in Settings.')}
				</p>
				<div class="mt-8 grid grid-cols-2 gap-3">
					<button
						class="rounded-xl border border-[#7aa2ff]/60 bg-[#7aa2ff]/15 px-6 py-5 transition hover:bg-[#7aa2ff]/25"
						on:click={() => pickLanguage('it-IT')}
						data-welcome-animate
					>
						<div class="text-lg">🇮🇹</div>
						<div class="mt-1 text-sm">Italiano</div>
					</button>
					<button
						class="rounded-xl border border-white/10 bg-white/5 px-6 py-5 transition hover:bg-white/10"
						on:click={() => pickLanguage('en-US')}
						data-welcome-animate
					>
						<div class="text-lg">🇬🇧</div>
						<div class="mt-1 text-sm">English</div>
					</button>
				</div>
			</div>
		{:else if step === 3}
			<div class="mx-auto max-w-xl">
				<h2 class="text-center text-2xl font-light tracking-tight" data-welcome-animate>
					{$i18n.t('Recommended models')}
				</h2>
				<p class="mt-2 text-center text-sm font-light text-white/60" data-welcome-animate>
					{#if ollamaOk}
						{$i18n.t('Ollama detected. One heavy model at a time works best on 8 GB VRAM.')}
					{:else}
						{$i18n.t(
							'Ollama was not detected. Install it, or serve the models with Unsloth Studio.'
						)}
					{/if}
				</p>
				<div class="mt-8 grid grid-cols-1 gap-3 sm:grid-cols-2">
					{#each RECOMMENDED as rec}
						<div class="rounded-xl border border-white/10 bg-white/5 p-5" data-welcome-animate>
							<div class="flex items-center justify-between">
								<div class="text-sm font-medium">{rec.title}</div>
								{#if hasModel(rec)}
									<span
										class="rounded-full bg-emerald-400/15 px-2.5 py-0.5 text-[0.6875rem] text-emerald-300"
										>● {$i18n.t('Ready')}</span
									>
								{:else}
									<span
										class="rounded-full bg-white/10 px-2.5 py-0.5 text-[0.6875rem] text-white/60"
										>○ {$i18n.t('Not installed')}</span
									>
								{/if}
							</div>
							<div class="mt-1 text-[0.6875rem] text-white/40">{rec.meta}</div>
							<p class="mt-3 text-xs leading-relaxed text-white/60">{$i18n.t(rec.desc)}</p>
						</div>
					{/each}
				</div>
				<p class="mt-4 text-center text-xs text-white/40" data-welcome-animate>
					{$i18n.t('You can install them later from Admin Settings. Nothing is mandatory.')}
				</p>
				<div class="mt-8 text-center" data-welcome-animate>
					<button
						class="bg-white px-10 py-3 text-sm text-black transition hover:bg-white/90"
						on:click={() => {
							step = 4;
						}}
					>
						{$i18n.t('Continue')}
					</button>
				</div>
			</div>
		{:else}
			<div class="mx-auto max-w-md text-center">
				<div class="text-5xl" data-welcome-animate>🎉</div>
				<h2 class="mt-6 text-2xl font-light tracking-tight" data-welcome-animate>
					{$i18n.t('All set!')}
				</h2>
				<p class="mt-2 text-sm font-light text-white/60" data-welcome-animate>
					{$i18n.t('BlaskUI is ready. Open a new chat and pick a model to begin.')}
				</p>
				<button
					class="mt-8 bg-white px-10 py-3 text-sm text-black transition hover:bg-white/90"
					on:click={() => goto('/')}
					data-welcome-animate
				>
					{$i18n.t('Start chatting')}
				</button>
			</div>
		{/if}
	</div>
</div>
