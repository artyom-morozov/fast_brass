<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { gameState } from '$lib/store';
	import type { DiscardEntry } from '$lib/types';

	const dispatch = createEventDispatcher();

	export let open = false;
	export let playerIndex: number | null = null;

	$: gs = $gameState;
	$: allEntries = (gs?.discard_history ?? []) as DiscardEntry[];
	$: playerEntries = playerIndex == null
		? allEntries
		: allEntries.filter(e => e.player_index === playerIndex);
	$: player = playerIndex == null
		? null
		: gs?.players.find(p => p.index === playerIndex) ?? null;

	let currentIdx = 0;
	let animDir: 'left' | 'right' | null = null;

	$: if (currentIdx > Math.max(0, playerEntries.length - 1)) {
		currentIdx = Math.max(0, playerEntries.length - 1);
	}

	function close() {
		open = false;
		dispatch('close');
	}

	function go(delta: number) {
		if (playerEntries.length <= 1) return;
		animDir = delta > 0 ? 'right' : 'left';
		currentIdx = (currentIdx + delta + playerEntries.length) % playerEntries.length;
		setTimeout(() => (animDir = null), 220);
	}

	function onWheel(evt: WheelEvent) {
		evt.preventDefault();
		if (Math.abs(evt.deltaY) < 5) return;
		go(evt.deltaY > 0 ? 1 : -1);
	}

	function cardTitle(entry: DiscardEntry): string {
		return `${entry.card_label}`;
	}
</script>

{#if open}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div
		class="overlay"
		tabindex="-1"
		on:click|self={close}
		on:wheel|passive={onWheel}
		on:keydown={(e) => e.key === 'Escape' && close()}
	>
		<div class="modal" role="dialog" aria-modal="true">
			<div class="header">
				<h2>
					Discard Pile
					{#if player}
						<span class="for-player" style="color:{player.color}"> — {player.name}</span>
					{/if}
				</h2>
				<button class="close-btn" on:click={close}>×</button>
			</div>

			{#if playerEntries.length === 0}
				<div class="empty">No discarded cards yet.</div>
			{:else}
				<div class="viewer">
					<button class="nav prev" on:click={() => go(-1)} aria-label="Previous card">◀</button>
					<div class="track">
						{#if playerEntries[currentIdx - 1]}
							<div class="side-card left">
								<div class="label">{playerEntries[currentIdx - 1].card_label}</div>
							</div>
						{/if}
						<div class="center-card" class:slide-left={animDir === 'left'} class:slide-right={animDir === 'right'}>
							<div class="card-label">{cardTitle(playerEntries[currentIdx])}</div>
							<div class="card-type">{playerEntries[currentIdx].card_type}</div>
							<div class="meta">
								<span>Round {playerEntries[currentIdx].round_in_phase + 1}</span>
								<span>Turn {playerEntries[currentIdx].turn_count + 1}</span>
								<span>#{playerEntries[currentIdx].order + 1}</span>
							</div>
						</div>
						{#if playerEntries[currentIdx + 1]}
							<div class="side-card right">
								<div class="label">{playerEntries[currentIdx + 1].card_label}</div>
							</div>
						{/if}
					</div>
					<button class="nav next" on:click={() => go(1)} aria-label="Next card">▶</button>
				</div>
				<div class="hint">Use arrows or mouse wheel to browse in discard order.</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1100;
	}
	.modal {
		background: #1e1e2e;
		border: 1px solid #334155;
		border-radius: 12px;
		width: min(840px, 94vw);
		padding: 18px;
	}
	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 14px;
	}
	.header h2 {
		margin: 0;
		font-size: 1.2rem;
		color: #e2e8f0;
	}
	.close-btn {
		background: transparent;
		color: #94a3b8;
		border: none;
		font-size: 1.8rem;
		cursor: pointer;
	}
	.empty {
		text-align: center;
		color: #94a3b8;
		padding: 28px 0;
	}
	.viewer {
		display: grid;
		grid-template-columns: 56px 1fr 56px;
		gap: 8px;
		align-items: center;
	}
	.track {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 14px;
		min-height: 220px;
	}
	.nav {
		height: 48px;
		border-radius: 999px;
		border: 1px solid #475569;
		background: #0f172a;
		color: #cbd5e1;
		cursor: pointer;
	}
	.center-card {
		width: min(420px, 70vw);
		min-height: 180px;
		background: linear-gradient(180deg, #111827, #0f172a);
		border: 2px solid #7c3aed;
		border-radius: 12px;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		transition: transform 0.2s ease, opacity 0.2s ease;
	}
	.center-card.slide-left {
		transform: translateX(-8px);
	}
	.center-card.slide-right {
		transform: translateX(8px);
	}
	.side-card {
		width: 140px;
		min-height: 110px;
		background: #0f172a;
		border: 1px solid #334155;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 8px;
		opacity: 0.65;
	}
	.label {
		font-size: 0.85rem;
		text-align: center;
		color: #cbd5e1;
	}
	.card-label {
		font-size: 1.45rem;
		font-weight: 700;
		color: #f8fafc;
	}
	.card-type {
		font-size: 0.9rem;
		color: #a5b4fc;
	}
	.meta {
		display: flex;
		gap: 10px;
		font-size: 0.82rem;
		color: #94a3b8;
	}
	.hint {
		text-align: center;
		margin-top: 10px;
		font-size: 0.8rem;
		color: #94a3b8;
	}
</style>
