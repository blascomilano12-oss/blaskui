<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { user, config, models } from '$lib/stores';
	import { getBackendConfig } from '$lib/apis';
	import { createChat, getChatList } from '$lib/apis/chats';
	import Spinner from '$lib/components/common/Spinner.svelte';

	let loaded = false;
	let chatId = '';
	let messages: { role: string; content: string }[] = [];
	let input = '';
	let sending = false;
	let previewHtml = '';

	const SYSTEM_PROMPT = `Sei un agente sviluppatore web specializzato. Il tuo compito è creare siti web completi partendo dalle richieste dell'utente.

REGOLE:
1. Prima di iniziare a codare, fai 2-3 domande mirate per capire: stile, colori, sezioni, funzionalità
2. Quando hai abbastanza informazioni, genera un file HTML completo con CSS e JS inline
3. Usa sempre artifact HTML completi e funzionanti
4. Il codice deve essere moderno, responsive e visivamente accattivante
5. Rispondi sempre in italiano

FORMATO RISPOSTA:
- Per le domande: testo normale
- per il codice: usa il formato \`\`\`html ... \`\`\` per i file completi`;

	onMount(async () => {
		if (!$user) {
			goto('/auth');
			return;
		}
		try {
			await config.set(await getBackendConfig());
		} catch {
			// backend non raggiungibile
		}
		loaded = true;
	});

	const sendMessage = async () => {
		if (!input.trim() || sending) return;
		sending = true;
		const userMsg = input.trim();
		messages = [...messages, { role: 'user', content: userMsg }];
		input = '';

		try {
			const res = await fetch('/api/v1/chat/completions', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					Authorization: `Bearer ${localStorage.token}`
				},
				body: JSON.stringify({
					model: $models[0]?.id || 'blask-minicpm-test:latest',
					messages: [
						{ role: 'system', content: SYSTEM_PROMPT },
						...messages
					],
					stream: false
				})
			});
			const data = await res.json();
			const assistantMsg = data.choices?.[0]?.message?.content || 'Errore nella risposta';
			messages = [...messages, { role: 'assistant', content: assistantMsg }];

			// Estrai HTML dalla risposta per l'anteprima
			const htmlMatch = assistantMsg.match(/```html\n([\s\S]*?)```/);
			if (htmlMatch) {
				previewHtml = htmlMatch[1];
			}
		} catch (e) {
			messages = [...messages, { role: 'assistant', content: `Errore: ${e}` }];
		}
		sending = false;
	};
</script>

<svelte:head>
	<title>Chat to Code — BlaskUI</title>
</svelte:head>

{#if !loaded}
	<div class="flex h-screen items-center justify-center">
		<Spinner className="size-8" />
	</div>
{:else}
	<div class="flex h-screen">
		<!-- Chat panel -->
		<div class="flex w-1/2 flex-col border-r border-gray-200 dark:border-gray-800">
			<div class="flex items-center gap-3 border-b border-gray-200 px-4 py-3 dark:border-gray-800">
				<h1 class="text-lg font-semibold">Chat to Code</h1>
				<span class="rounded-full bg-blue-100 px-2 py-0.5 text-xs text-blue-700 dark:bg-blue-900/30 dark:text-blue-300">
					Agente Web
				</span>
			</div>
			<div class="flex-1 overflow-y-auto p-4 space-y-3" id="code-chat-messages">
				{#each messages as msg}
					<div
						class="rounded-xl p-4 {msg.role === 'user'
							? 'ml-8 bg-blue-600 text-white'
							: 'mr-8 bg-gray-100 dark:bg-gray-800'}"
					>
						<pre class="whitespace-pre-wrap text-sm font-sans">{msg.content}</pre>
					</div>
				{/each}
				{#if sending}
					<div class="mr-8 rounded-xl bg-gray-100 p-4 dark:bg-gray-800">
						<Spinner className="size-4" />
					</div>
				{/if}
			</div>
			<div class="border-t border-gray-200 p-4 dark:border-gray-800">
				<textarea
					class="w-full rounded-xl border border-gray-300 bg-transparent p-3 text-sm outline-none focus:border-blue-500 dark:border-gray-700"
					rows="3"
					placeholder="Descrivi il sito che vuoi creare..."
					bind:value={input}
					on:keydown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(); } }}
				></textarea>
				<button
					class="mt-2 w-full rounded-xl bg-blue-600 py-2.5 text-sm font-medium text-white transition hover:bg-blue-700 disabled:opacity-50"
					disabled={sending || !input.trim()}
					on:click={sendMessage}
				>
					{sending ? 'Invio...' : 'Invia'}
				</button>
			</div>
		</div>

		<!-- Preview panel -->
		<div class="flex w-1/2 flex-col">
			<div class="flex items-center justify-between border-b border-gray-200 px-4 py-3 dark:border-gray-800">
				<h2 class="text-sm font-medium text-gray-600 dark:text-gray-400">Anteprima Live</h2>
				{#if previewHtml}
					<button
						class="rounded-lg bg-gray-100 px-3 py-1.5 text-xs font-medium text-gray-700 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-300 dark:hover:bg-gray-700"
						on:click={() => { window.open('', '_blank').document.write(previewHtml); }}
					>
						Apri in nuova scheda
					</button>
				{/if}
			</div>
			<div class="flex-1 bg-white dark:bg-gray-900">
				{#if previewHtml}
					<iframe title="preview" class="h-full w-full border-0" srcdoc={previewHtml}></iframe>
				{:else}
					<div class="flex h-full items-center justify-center text-gray-400">
						<p>L'anteprima apparirà qui</p>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
