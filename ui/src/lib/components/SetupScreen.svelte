<script lang="ts">
	import { onMount } from 'svelte';
	import { listSavedGames, loadGame, newGame } from '$lib/api';
	import type { SavedGameSummary } from '$lib/api';
	import { createEventDispatcher } from 'svelte';

	const dispatch = createEventDispatcher();
	let activeTab: 'new' | 'join' = 'new';
	let numPlayers = 4;
	let seedInput = '';
	let seedError = '';
	let savedGames: SavedGameSummary[] = [];
	let loadingGames = false;
	let loadingError = '';

	function formatCreatedAt(createdAt: number | string) {
		const fromString = typeof createdAt === 'string' ? Number(createdAt) : createdAt;
		const numeric = Number.isFinite(fromString) ? fromString : NaN;
		const millis = Number.isFinite(numeric)
			? (numeric > 1_000_000_000_000 ? numeric : numeric * 1000)
			: NaN;
		const parsed = Number.isFinite(millis) ? new Date(millis) : new Date(createdAt);
		return Number.isNaN(parsed.getTime()) ? String(createdAt) : parsed.toLocaleString();
	}

	async function refreshSavedGames() {
		loadingGames = true;
		loadingError = '';
		try {
			savedGames = await listSavedGames();
		} catch {
			loadingError = 'Failed to load saved games';
			savedGames = [];
		} finally {
			loadingGames = false;
		}
	}

	function switchTab(tab: 'new' | 'join') {
		activeTab = tab;
		if (tab === 'join') {
			void refreshSavedGames();
		}
	}

	async function startNewGame() {
		seedError = '';
		let seed: number | null = null;
		const raw = seedInput.trim();
		if (raw.length > 0) {
			const n = Number(raw);
			if (!Number.isInteger(n) || n < 0 || n > Number.MAX_SAFE_INTEGER) {
				seedError = 'Seed must be a non-negative integer';
				return;
			}
			seed = n;
		}
		const data = await newGame(numPlayers, seed);
		if (data) dispatch('started');
	}

	async function joinSavedGame(gameId: number) {
		const data = await loadGame(gameId);
		if (data) dispatch('started');
	}

	onMount(() => {
		void refreshSavedGames();
	});
</script>

<div class="setup">
	<h1>Brass Birmingham</h1>

	<div class="tabs">
		<button class:active={activeTab === 'new'} on:click={() => switchTab('new')}>New Game</button>
		<button class:active={activeTab === 'join'} on:click={() => switchTab('join')}>Join Game</button>
	</div>

	{#if activeTab === 'new'}
		<div class="row">
			<label for="np">Players</label>
			<select id="np" bind:value={numPlayers}>
				<option value={2}>2 Players</option>
				<option value={3}>3 Players</option>
				<option value={4}>4 Players</option>
			</select>
		</div>
		<div class="row">
			<label for="seed">Seed (optional)</label>
			<input id="seed" type="text" bind:value={seedInput} placeholder="Random if empty" />
			{#if seedError}
				<div class="seed-error">{seedError}</div>
			{/if}
		</div>
		<button class="btn" on:click={startNewGame}>Start New Game</button>
	{:else}
		<div class="join-list">
			{#if loadingGames}
				<div class="join-note">Loading saved games...</div>
			{:else if loadingError}
				<div class="join-note error">{loadingError}</div>
			{:else if savedGames.length === 0}
				<div class="join-note">No saved games found.</div>
			{:else}
				{#each savedGames as game}
					<div class="game-row">
						<div class="game-meta">
							<div class="game-title">{formatCreatedAt(game.created_at)}</div>
							<div class="game-details">
								Round {game.round_in_phase + 1} | {game.era} | {game.num_players}p | Seed {game.seed}
							</div>
						</div>
						<button class="btn join-btn" on:click={() => joinSavedGame(game.id)}>Join</button>
					</div>
				{/each}
			{/if}
		</div>
	{/if}
</div>

<style>
	.setup {
		max-width: 380px;
		margin: 100px auto;
		text-align: center;
		background: #16213e;
		padding: 40px;
		border-radius: 12px;
		border: 1px solid #2a3a5c;
	}
	h1 { font-size: 28px; color: #c7a750; margin-bottom: 24px; }
	.row { margin-bottom: 16px; }
	.tabs {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
		margin-bottom: 18px;
	}
	.tabs button {
		padding: 8px 10px;
		border-radius: 8px;
		border: 1px solid #334155;
		background: #0f172a;
		color: #cbd5e1;
		cursor: pointer;
		font-weight: 600;
	}
	.tabs button.active {
		background: #1e40af;
		border-color: #2563eb;
		color: #fff;
	}
	label { display: block; margin-bottom: 6px; color: #94a3b8; font-size: 14px; }
	select {
		padding: 8px 20px; border-radius: 6px; border: 1px solid #475569;
		background: #1e293b; color: #e2e8f0; font-size: 14px;
	}
	input {
		padding: 8px 12px; border-radius: 6px; border: 1px solid #475569;
		background: #1e293b; color: #e2e8f0; font-size: 14px; width: 100%;
		box-sizing: border-box;
	}
	.seed-error { color: #ef4444; font-size: 12px; margin-top: 6px; }
	.btn {
		padding: 10px 32px; border: none; border-radius: 8px; cursor: pointer;
		font-size: 15px; font-weight: 700; color: #fff; background: #2563eb;
		transition: background 0.15s;
	}
	.btn:hover { background: #1d4ed8; }
	.join-list {
		display: grid;
		gap: 10px;
		text-align: left;
	}
	.join-note {
		color: #cbd5e1;
		font-size: 14px;
		padding: 10px 12px;
		background: #0f172a;
		border-radius: 8px;
		border: 1px solid #334155;
	}
	.join-note.error { color: #fca5a5; }
	.game-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		border-radius: 8px;
		border: 1px solid #334155;
		background: #0f172a;
	}
	.game-meta { min-width: 0; }
	.game-title {
		color: #e2e8f0;
		font-size: 14px;
		font-weight: 700;
	}
	.game-details {
		color: #94a3b8;
		font-size: 12px;
		margin-top: 2px;
	}
	.join-btn {
		padding: 8px 14px;
		font-size: 13px;
		white-space: nowrap;
	}
</style>
