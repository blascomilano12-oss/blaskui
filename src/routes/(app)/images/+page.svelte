<script lang="ts">
	import { onMount, getContext } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { goto } from '$app/navigation';
	import { user, config } from '$lib/stores';
	import { imageGenerations, imageEdits, getConfig as getImageConfig } from '$lib/apis/images';
	import { searchFiles, getFiles } from '$lib/apis/files';
	import Spinner from '$lib/components/common/Spinner.svelte';
	import { WEBUI_API_BASE_URL } from '$lib/constants';

	const i18n: any = getContext('i18n');

	let prompt = '';
	let loading = false;
	let generating = false;
	let gallery: any[] = [];
	let galleryPage = 1;
	let galleryLoading = false;
	let searchQuery = 'Search low poly art';
	let activeTab: 'Images' | 'Moodboards' = 'Images';
	let selectedPreset: string | null = null;

	// preset stili come da screenshot 1 — placeholder con gradient, poi vere immagini quando ci sono dati
	const presets = [
		{ id: 'sketch', label: 'Sketch', prompt: 'minimal line sketch of a flower, white background, clean vector, 2d illustration', gradient: 'from-white to-gray-100', icon: '🌼' },
		{ id: '80s', label: "'80s flashback", prompt: '80s flashback portrait, synthwave, warm grainy film, 1980s fashion, analog photo', gradient: 'from-orange-200 to-pink-300', icon: '🕶️' },
		{ id: 'adesivi', label: 'Adesivi', prompt: 'cute kawaii stickers sheet, die-cut, white background, chibi style', gradient: 'from-yellow-100 to-green-100', icon: '😺' },
		{ id: 'caricatura', label: 'Crea una caricatura', prompt: 'caricature of a person, exaggerated features, cartoon 3d, vibrant', gradient: 'from-blue-200 to-purple-200', icon: '🧑‍🎨' },
		{ id: 'anime', label: 'Anime', prompt: 'anime portrait, studio ghibli style, soft lighting, detailed', gradient: 'from-sky-200 to-indigo-300', icon: '✨' }
	];

	let imageConfig: any = null;

	const PRESET_IMAGES: Record<string, string> = {
		sketch: 'https://images.unsplash.com/photo-1579783902614-a3fb3927b6a5?w=400&h=400&fit=crop',
		'80s': 'https://images.unsplash.com/photo-1487412720507-e7ab37603c6f?w=400&h=500&fit=crop',
		adesivi: 'https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=400&h=400&fit=crop',
		caricatura: 'https://images.unsplash.com/photo-1535713875005-d1d0cf377fde?w=400&h=500&fit=crop',
		anime: 'https://images.unsplash.com/photo-1578632292335-df3ab3100d00?w=400&h=500&fit=crop'
	};

	onMount(async () => {
		if ($user?.role !== 'admin' && !$user) {
			// allow all users to see gallery, but generation needs permission
		}
		try {
			imageConfig = await getImageConfig(localStorage.token);
		} catch {}
		await loadGallery(true);
	});

	const loadGallery = async (reset = false) => {
		if (reset) {
			galleryPage = 1;
			gallery = [];
		}
		galleryLoading = true;
		try {
			// ponytail: riusa files API già presente, filtra generated-image*
			const res: any = await searchFiles(localStorage.token, 'generated-image*', (galleryPage - 1) * 50, 50, false).catch(() => null);
			if (res && Array.isArray(res)) {
				const imgs = res.filter((f: any) => (f.meta?.content_type ?? '').startsWith('image/') || f.filename?.startsWith('generated-image'));
				gallery = [...gallery, ...imgs];
			} else {
				const all: any = await getFiles(localStorage.token, false).catch(() => ({ files: [] }));
				const items = all?.files ?? all ?? [];
				const imgs = items.filter((f: any) => (f.meta?.content_type ?? '').startsWith('image/') || f.filename?.includes('generated-image')).slice((galleryPage - 1) * 50, galleryPage * 50);
				if (reset) gallery = imgs;
				else gallery = [...gallery, ...imgs];
			}
		} catch (e) {
			console.error(e);
		} finally {
			galleryLoading = false;
		}
	};

	const handlePreset = (p: typeof presets[0]) => {
		selectedPreset = p.id;
		prompt = p.prompt;
	};

	const handleGenerate = async () => {
		if (!prompt.trim()) {
			toast.error('Inserisci un prompt');
			return;
		}
		generating = true;
		try {
			const result = await imageGenerations(localStorage.token, prompt);
			if (result && Array.isArray(result) && result.length > 0) {
				// prepend to gallery immediately (ottimistico)
				gallery = [...result.map((r: any) => ({ id: r.id, url: r.url, filename: r.name, meta: { content_type: r.content_type ?? 'image/png' } })), ...gallery];
				toast.success('Immagine creata');
				// ricarica da backend per coerenza
				setTimeout(() => loadGallery(true), 800);
			} else {
				toast.success('Generazione avviata');
			}
		} catch (e) {
			toast.error(`${e}`);
		} finally {
			generating = false;
		}
	};

	const handleScroll = (e: Event) => {
		const el = e.target as HTMLElement;
		if (el.scrollTop + el.clientHeight >= el.scrollHeight - 400 && !galleryLoading) {
			galleryPage += 1;
			loadGallery(false);
		}
	};

	const downloadImage = async (url: string) => {
		try {
			const full = url.startsWith('/') ? `${WEBUI_API_BASE_URL}${url}`.replace('/api/v1', '') : url;
			// prova via files content
			const fileUrl = url.startsWith('/api') ? url : `${WEBUI_API_BASE_URL}/files/${url}/content`;
			// se è già url completo con /api/v1/files
			const target = url.startsWith('/api') ? `${window.location.origin}${url}` : fileUrl;
			const res = await fetch(target, { headers: { Authorization: `Bearer ${localStorage.token}` } });
			const blob = await res.blob();
			const a = document.createElement('a');
			a.href = URL.createObjectURL(blob);
			a.download = `blaskui-${Date.now()}.png`;
			a.click();
			URL.revokeObjectURL(a.href);
		} catch {}
	};

	const filteredGallery = gallery;
</script>

<div class="flex h-full w-full flex-col bg-black/40 text-white overflow-hidden relative backdrop-blur-2xl">
	<!-- header glass -->
	<div class="shrink-0 sticky top-0 z-10 backdrop-blur-xl bg-black/40 border-b border-white/10">
		<div class="mx-auto max-w-[1600px] px-4 md:px-6 py-4">
			<div class="flex items-center justify-between">
				<h1 class="text-lg font-medium tracking-tight">Immagini</h1>
				<div class="hidden md:flex items-center gap-2 text-xs text-white/50">
					<span>ComfyUI: Z-Image Turbo</span>
					<span class="h-1 w-1 rounded-full bg-white/20"></span>
					<span>HF: RealVisXL_V4.0</span>
				</div>
			</div>
		</div>
	</div>

	<div class="flex-1 overflow-y-auto scrollbar-hidden" on:scroll={handleScroll}>
		<div class="mx-auto max-w-[980px] px-4 md:px-6 pt-6 pb-10">
			<!-- Prompt input glass (come screenshot1) -->
			<div class="rounded-2xl bg-black/40 backdrop-blur-xl border border-white/10 p-3 md:p-4">
				<textarea
					bind:value={prompt}
					placeholder="lucina verde accesa tutti gli arancioni nascondili quando non disponibili non si vedono è una regola. inoltre mi"
					rows="3"
					class="w-full resize-none bg-transparent text-sm text-white placeholder:text-white/40 focus:outline-none"
				></textarea>
				<div class="mt-3 flex items-center justify-between">
					<button class="flex size-7 items-center justify-center rounded-full bg-white/5 border border-white/10 text-white/60 hover:bg-white/10 transition">
						<svg class="size-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13"/></svg>
					</button>
					<div class="flex items-center gap-2">
						<button class="flex size-7 items-center justify-center rounded-full bg-white/5 border border-white/10 text-white/60">
							<svg class="size-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z"/></svg>
						</button>
						<button
							on:click={handleGenerate}
							disabled={generating}
							class="flex size-8 items-center justify-center rounded-full bg-white text-black hover:bg-white/90 transition disabled:opacity-50"
						>
							{#if generating}<Spinner className="size-4" />{:else}<svg class="size-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"/></svg>{/if}
						</button>
					</div>
				</div>
			</div>

			<!-- Preset carousel -->
			<div class="mt-6">
				<div class="flex items-center justify-between">
					<h2 class="text-sm font-medium text-white">Crea un'immagine</h2>
					<div class="flex gap-1">
						<button class="size-6 rounded-full bg-white/5 border border-white/10 flex items-center justify-center text-white/60"><svg class="size-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 19l-7-7 7-7"/></svg></button>
						<button class="size-6 rounded-full bg-white/5 border border-white/10 flex items-center justify-center text-white/60"><svg class="size-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5l7 7-7 7"/></svg></button>
					</div>
				</div>
				<div class="mt-3 flex gap-3 overflow-x-auto scrollbar-none snap-x snap-mandatory pb-2">
					{#each presets as p}
						<button
							on:click={() => handlePreset(p)}
							class="shrink-0 snap-start group relative h-[140px] w-[105px] overflow-hidden rounded-xl border {selectedPreset === p.id ? 'border-white/40 ring-1 ring-white/20' : 'border-white/10'} bg-black/40 backdrop-blur-xl hover:border-white/20 transition"
						>
							<img src={PRESET_IMAGES[p.id]} alt={p.label} class="h-full w-full object-cover opacity-90 group-hover:opacity-100 transition" loading="lazy" />
							<div class="absolute inset-0 bg-gradient-to-t from-black/70 via-black/10 to-transparent"></div>
							<span class="absolute bottom-1.5 left-2 right-1 text-[10px] leading-tight font-medium text-white drop-shadow">{p.label}</span>
						</button>
					{/each}
				</div>
			</div>

			<!-- Gallery header -->
			<div class="mt-10 sticky top-0 z-5 backdrop-blur-xl bg-black/40 -mx-4 md:-mx-6 px-4 md:px-6 py-3 border-y border-white/5">
				<div class="flex items-center justify-between">
					<div class="flex gap-4 text-xs">
						<button class="font-medium text-white border-b border-white pb-1" on:click={() => (activeTab = 'Images')}>Images</button>
						<button class="text-white/40 hover:text-white/70" on:click={() => (activeTab = 'Moodboards')}>Moodboards</button>
					</div>
					<div class="relative hidden md:block">
						<input
							bind:value={searchQuery}
							placeholder="Search low poly art"
							class="w-[260px] rounded-full bg-white/5 border border-white/10 pl-8 pr-3 py-1.5 text-xs text-white placeholder:text-white/30 focus:outline-none focus:border-white/20"
						/>
						<svg class="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-white/40" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
					</div>
				</div>
			</div>

			<!-- Masonry gallery -->
			<div class="mt-4 columns-2 md:columns-3 lg:columns-5 gap-3 space-y-3">
				{#each filteredGallery as img, i}
					<button
						on:click={() => downloadImage(img.url ?? img.id)}
						class="break-inside-avoid group relative block w-full overflow-hidden rounded-xl bg-black/40 backdrop-blur-xl border border-white/5 hover:border-white/15 transition"
					>
						<img
							src={img.url?.startsWith('/api') ? `${WEBUI_API_BASE_URL}${img.url}`.replace('/api/v1', '') : img.url ?? `${WEBUI_API_BASE_URL}/files/${img.id}/content`}
							alt={img.filename ?? 'generated'}
							class="w-full h-auto object-cover"
							loading="lazy"
						/>
						<div class="absolute inset-0 opacity-0 group-hover:opacity-100 bg-black/30 backdrop-blur-[1px] transition flex items-center justify-center">
							<span class="rounded-full bg-white text-black text-xs px-3 py-1">Apri</span>
						</div>
					</button>
				{:else}
					<!-- placeholder quando vuota: mostra le immagini demo Krea per far capire lo stile -->
					{#each Array(15) as _, idx}
						<div class="break-inside-avoid relative overflow-hidden rounded-xl bg-gradient-to-br from-gray-800 to-gray-900 border border-white/5 aspect-[{idx % 3 === 0 ? '3/4' : idx % 3 === 1 ? '1/1' : '4/3'}] flex items-center justify-center">
							<span class="text-white/20 text-xs">Demo {idx + 1}</span>
						</div>
					{/each}
				{/each}
			</div>

			{#if galleryLoading}
				<div class="flex justify-center py-8"><Spinner className="size-5" /></div>
			{/if}

			<div class="mt-8 rounded-xl border border-white/10 bg-white/[0.04] backdrop-blur-xl p-4 text-xs text-white/60">
				Tip: scrivi “genera immagine fotorealistica di...” nella chat normale con <span class="text-white">✨ Immagini</span> attivo, oppure usa i preset sopra. Z-Image Turbo è locale e gratuito, HF RealVisXL è cloud free.
			</div>
		</div>
	</div>
</div>

<style>
	.scrollbar-none::-webkit-scrollbar {
		display: none;
	}
	.scrollbar-none {
		-ms-overflow-style: none;
		scrollbar-width: none;
	}
</style>
