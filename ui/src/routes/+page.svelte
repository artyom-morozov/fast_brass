<script lang="ts">
	import { gameState, choiceSet, turnPhase } from '$lib/store';
	import SetupScreen from '$lib/components/SetupScreen.svelte';
	import Board from '$lib/components/Board.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import CardHand from '$lib/components/CardHand.svelte';
	import IndustryMat from '$lib/components/IndustryMat.svelte';
	import DiscardPileViewer from '$lib/components/DiscardPileViewer.svelte';

	let started = false;
	let matOpen = false;
	let matPlayerIndex: number | null = null;
	let discardOpen = false;
	let discardPlayerIndex: number | null = null;

	$: gs = $gameState;
	$: cs = $choiceSet;
	$: isCardChoice = cs?.kind === 'card';
</script>

{#if !started}
	<SetupScreen on:started={() => started = true} />
{:else if gs}
	<div class="game-layout">
		<div class="left-col">
			<div class="board-area">
				<Board />
			</div>
			<div class="bottom-bar" class:card-active={isCardChoice}>
				{#if $turnPhase === 'in_session' && isCardChoice}
					<div class="card-prompt">Pick a card to discard</div>
				{/if}
				<CardHand />
			</div>
		</div>
		<div class="right-col">
			<Sidebar
				on:openMat={(e) => {
					matPlayerIndex = e.detail?.playerIndex ?? null;
					matOpen = true;
				}}
				on:openDiscard={(e) => {
					discardPlayerIndex = e.detail?.playerIndex ?? null;
					discardOpen = true;
				}}
			/>
		</div>
	</div>
	<IndustryMat bind:open={matOpen} playerIndex={matPlayerIndex} on:close={() => matOpen = false} />
	<DiscardPileViewer bind:open={discardOpen} playerIndex={discardPlayerIndex} on:close={() => discardOpen = false} />
{/if}

<style>
	.game-layout {
		display: flex;
		height: 100vh;
		overflow: hidden;
	}
	.left-col {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.board-area {
		flex: 1;
		overflow: hidden;
		position: relative;
		min-height: 0;
	}
	.bottom-bar {
		flex-shrink: 0;
		background: #0f172a;
		border-top: 1px solid #2a3a5c;
		padding: 4px 16px;
		min-height: 130px;
		transition: background 0.3s;
	}
	.bottom-bar.card-active {
		background: #1e1b4b;
		border-top-color: #7c3aed;
	}
	.card-prompt {
		text-align: center;
		color: #a78bfa;
		font-size: 13px;
		font-weight: 600;
		margin-bottom: 2px;
	}
	.right-col {
		width: 300px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		border-left: 1px solid #2a3a5c;
	}
</style>
