<script lang="ts">
	import { onMount, getContext, tick } from 'svelte';
	import type { Readable } from 'svelte/store';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { animate } from 'motion/mini';

	import { getBackendConfig } from '$lib/apis';
	import { userSignIn, userSignUp, addUser, getSessionUser } from '$lib/apis/auths';
	import { config, user } from '$lib/stores';
	import { generateInitialsImage, getUserTimezone } from '$lib/utils';
	import { updateUserTimezone } from '$lib/apis/auths';
	import {
		getBlaskAccounts,
		rememberBlaskAccount,
		forgetBlaskAccount,
		getBlaskAdminToken,
		accountHue,
		accountInitials,
		type BlaskAccount
	} from '$lib/utils/accounts';
	import Spinner from '$lib/components/common/Spinner.svelte';

	const i18n = getContext<Readable<{ t: (key: string, opts?: object) => string }>>('i18n');

	let loaded = false;
	let accounts: BlaskAccount[] = [];
	let selected: BlaskAccount | null = null;
	let password = '';
	let submitting = false;

	let creating = false;
	let newName = '';
	let newEmail = '';
	let newPassword = '';

	const pop = async () => {
		await tick();
		animate(
			'[data-accounts-animate]',
			{ opacity: [0, 1], y: [18, 0] },
			{ duration: 0.4, delay: 0.05, easing: 'ease-out' }
		);
	};

	const finishLogin = async (sessionUser, redirectPath: string | null = null) => {
		if (sessionUser?.token) localStorage.token = sessionUser.token;
		await user.set(sessionUser);
		await config.set(await getBackendConfig());
		rememberBlaskAccount(sessionUser);
		const timezone = getUserTimezone();
		if (sessionUser.token && timezone) updateUserTimezone(sessionUser.token, timezone);
		goto(redirectPath ?? '/');
	};

	const signInHandler = async () => {
		if (!selected || !password) {
			toast.error($i18n.t('Please fill in all fields.'));
			return;
		}
		submitting = true;
		try {
			const sessionUser = await userSignIn(selected.email, password).catch((error) => {
				toast.error(`${error}`);
				return null;
			});
			if (sessionUser) {
				toast.success($i18n.t(`You're now logged in.`));
				await finishLogin(sessionUser);
			}
		} finally {
			submitting = false;
		}
	};

	const createHandler = async () => {
		if (!newName.trim() || !newEmail.trim() || !newPassword) {
			toast.error($i18n.t('Please fill in all fields.'));
			return;
		}
		submitting = true;
		try {
			if ($config?.features?.enable_signup) {
				const sessionUser = await userSignUp(
					newName.trim(),
					newEmail.trim(),
					newPassword,
					generateInitialsImage(newName.trim())
				).catch((error) => {
					toast.error(`${error}`);
					return null;
				});
				if (sessionUser) {
					toast.success($i18n.t('Account created. Welcome to BlaskUI!'));
					await finishLogin(sessionUser);
				}
				return;
			}
			const adminToken = getBlaskAdminToken();
			if (!adminToken) {
				toast.error(
					$i18n.t('Signups are closed. Ask an admin to create the account first.')
				);
				return;
			}
			const created = await addUser(
				adminToken,
				newName.trim(),
				newEmail.trim(),
				newPassword,
				'user',
				generateInitialsImage(newName.trim())
			).catch((error) => {
				toast.error(`${error}`);
				return null;
			});
			if (created) {
				toast.success($i18n.t('Profile created. Sign in now.'));
				creating = false;
				newName = newEmail = newPassword = '';
				accounts = getBlaskAccounts();
				selected = {
					name: created.name ?? newName.trim(),
					email: created.email ?? newEmail.trim()
				};
				await pop();
			}
		} finally {
			submitting = false;
		}
	};

	onMount(async () => {
		if ($user) {
			goto('/');
			return;
		}
		try {
			await config.set(await getBackendConfig());
		} catch {
			// backend non raggiungibile: mostra comunque la pagina
		}
		accounts = getBlaskAccounts();
		loaded = true;
		await pop();
	});
</script>

<svelte:head>
	<title>{$i18n.t('Who is using BlaskUI?')}</title>
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
	<div class="absolute inset-0 bg-black/60"></div>

	<div class="relative w-full max-w-3xl px-6 py-12">
		{#if !loaded}
			<div class="flex justify-center py-16"><Spinner className="size-8" /></div>
		{:else if selected && !creating}
			<!-- stile macbook: avatar grande + password -->
			<div class="mx-auto max-w-xs text-center">
				<button
					class="mb-6 text-xs text-white/50 transition hover:text-white"
					on:click={() => {
						selected = null;
						password = '';
					}}
					data-accounts-animate
				>
					← {$i18n.t('All profiles')}
				</button>
				<div
					class="mx-auto flex size-24 items-center justify-center rounded-full text-3xl font-semibold"
					style="background: linear-gradient(135deg, hsl({accountHue(
						selected.name
					)} 70% 45%), hsl({(accountHue(selected.name) + 60) % 360} 70% 35%));"
					data-accounts-animate
				>
					{accountInitials(selected.name)}
				</div>
				<h2 class="mt-4 text-xl font-light" data-accounts-animate>{selected.name}</h2>
				<form
					class="mt-6 flex flex-col gap-3"
					on:submit|preventDefault={signInHandler}
					data-accounts-animate
				>
					<input
						type="password"
						class="rounded-full border border-white/15 bg-white/10 px-5 py-2.5 text-center text-sm text-white outline-none backdrop-blur placeholder:text-white/40 focus:border-white/40"
						placeholder={$i18n.t('Password')}
						bind:value={password}
						autocomplete="current-password"
					/>
					<button
						type="submit"
						disabled={submitting}
						class="flex items-center justify-center gap-2 rounded-full bg-white px-8 py-2.5 text-sm text-black transition hover:bg-white/90 disabled:opacity-60"
					>
						{#if submitting}<Spinner className="size-4" />{/if}
						{$i18n.t('Sign in')}
					</button>
				</form>
			</div>
		{:else if creating}
			<div class="mx-auto max-w-md">
				<button
					class="mb-6 text-xs text-white/50 transition hover:text-white"
					on:click={() => {
						creating = false;
					}}
					data-accounts-animate
				>
					← {$i18n.t('All profiles')}
				</button>
				<h2 class="text-2xl font-light tracking-tight" data-accounts-animate>
					{$i18n.t('Create a new profile')}
				</h2>
				<form class="mt-6 flex flex-col gap-3" on:submit|preventDefault={createHandler} data-accounts-animate>
					<input
						class="rounded-xl border border-white/10 bg-white/10 px-4 py-2.5 text-sm text-white outline-none backdrop-blur placeholder:text-white/40 focus:border-white/40"
						placeholder={$i18n.t('Name')}
						bind:value={newName}
						autocomplete="name"
					/>
					<input
						type="email"
						class="rounded-xl border border-white/10 bg-white/10 px-4 py-2.5 text-sm text-white outline-none backdrop-blur placeholder:text-white/40 focus:border-white/40"
						placeholder="Email"
						bind:value={newEmail}
						autocomplete="email"
					/>
					<input
						type="password"
						class="rounded-xl border border-white/10 bg-white/10 px-4 py-2.5 text-sm text-white outline-none backdrop-blur placeholder:text-white/40 focus:border-white/40"
						placeholder={$i18n.t('Password')}
						bind:value={newPassword}
						autocomplete="new-password"
					/>
					<button
						type="submit"
						disabled={submitting}
						class="mt-2 flex items-center justify-center gap-2 rounded-xl bg-white px-8 py-2.5 text-sm text-black transition hover:bg-white/90 disabled:opacity-60"
					>
						{#if submitting}<Spinner className="size-4" />{/if}
						{$i18n.t('Create profile')}
					</button>
				</form>
			</div>
		{:else}
			<!-- stile netflix: griglia profili -->
			<div class="text-center">
				<img src="/assets/brand/blaskui-logo-animated.svg" class="mx-auto h-9 w-auto" alt="BlaskUI" data-accounts-animate />
				<h1 class="mt-4 text-2xl font-light tracking-tight lg:text-3xl" data-accounts-animate>
					{$i18n.t('Who is using BlaskUI?')}
				</h1>
				<div class="mt-10 flex flex-wrap items-start justify-center gap-6">
					{#each accounts as account}
						<button
							class="group flex w-28 flex-col items-center gap-3"
							on:click={() => {
								selected = account;
								password = '';
							}}
							data-accounts-animate
						>
							<div
								class="flex size-24 items-center justify-center rounded-2xl text-2xl font-semibold transition group-hover:scale-105 group-hover:ring-2 group-hover:ring-white/70"
								style="background: linear-gradient(135deg, hsl({accountHue(
									account.name
								)} 70% 45%), hsl({(accountHue(account.name) + 60) % 360} 70% 35%));"
							>
								{accountInitials(account.name)}
							</div>
							<span class="max-w-full truncate text-sm text-white/70 group-hover:text-white">
								{account.name}
							</span>
						</button>
					{/each}
					<button
						class="group flex w-28 flex-col items-center gap-3"
						on:click={() => {
							creating = true;
						}}
						data-accounts-animate
					>
						<div
							class="flex size-24 items-center justify-center rounded-2xl border border-dashed border-white/30 text-3xl text-white/60 transition group-hover:scale-105 group-hover:border-white/70 group-hover:text-white"
						>
							+
						</div>
						<span class="text-sm text-white/70 group-hover:text-white">{$i18n.t('Add profile')}</span>
					</button>
				</div>
				{#if accounts.length === 0}
					<p class="mx-auto mt-6 max-w-md text-xs text-white/50" data-accounts-animate>
						{$i18n.t('No saved profiles on this machine yet. Sign in once and you will find it here.')}
					</p>
				{/if}
				<div class="mt-10" data-accounts-animate>
					<a href="/auth" class="text-xs text-white/50 underline transition hover:text-white">
						{$i18n.t('Classic sign in')}
					</a>
				</div>
			</div>
		{/if}
	</div>
</div>
