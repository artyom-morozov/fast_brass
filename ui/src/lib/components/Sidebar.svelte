<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { gameState, turnPhase, actionsAvailable, choiceSet, logs, moneyAtTurnStart, pendingDevelopments, actionBudgetAtTurnStart } from '$lib/store';
	import { INDUSTRY_COLORS } from '$lib/coords';
	import { startTurn, selectAction, applyChoice, confirmAction, cancelAction, endTurn, undoLastAction } from '$lib/api';

	const dispatch = createEventDispatcher();

	$: gs = $gameState;
	$: cs = $choiceSet;
	$: phase = $turnPhase;
	$: actions = $actionsAvailable;
	$: moneySnap = $moneyAtTurnStart;
	$: pendingDevs = $pendingDevelopments;
	$: actionBudget = $actionBudgetAtTurnStart;
	let roundToast: string | null = null;
	let toastTimer: ReturnType<typeof setTimeout> | null = null;
	let prevRound = -1;
	let prevEra = '';
let seedCopied = false;
let seedCopyTimer: ReturnType<typeof setTimeout> | null = null;

	$: orderedPlayers = gs ? gs.turn_order.map(idx => gs!.players.find(p => p.index === idx)!).filter(Boolean) : [];

	const LEVEL_ROMAN = ['I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII'];
	function levelToRoman(level: number): string {
		return LEVEL_ROMAN[level - 1] ?? String(level);
	}
	function devIconPath(industry: string): string {
		return `/assets/buildings/icons/${industry.toLowerCase()}.svg`;
	}

	function roundsPerEra(numPlayers: number): number {
		if (numPlayers === 2) return 10;
		if (numPlayers === 3) return 9;
		return 8;
	}

	function currentActionNumber(): number {
		if (!gs) return 1;
		const n = actionBudget - gs.actions_remaining + 1;
		return Math.min(Math.max(1, n), actionBudget);
	}

	function moneyDelta(pIdx: number, currentMoney: number): string {
		if (moneySnap[pIdx] == null) return '';
		const d = moneySnap[pIdx] - currentMoney;
		if (d > 0) return ` (-£${d})`;
		if (d < 0) return ` (+£${-d})`;
		return '';
	}

	const ACTION_LABELS: Record<string, string> = {
		BuildBuilding: 'Build', BuildRailroad: 'Network', BuildDoubleRailroad: 'Double Rail',
		Develop: 'Develop', DevelopDouble: 'Develop x2', Sell: 'Sell',
		Loan: 'Loan', Scout: 'Scout', Pass: 'Pass'
	};
	const ACTION_EMOJI: Record<string, string> = {
		BuildBuilding: '🏗️', BuildRailroad: '🛤️', BuildDoubleRailroad: '🚆',
		Develop: '🔧', DevelopDouble: '⚙️', Sell: '💰',
		Loan: '🏦', Scout: '🧭', Pass: '⏭️'
	};
	const CHOICE_TITLES: Record<string, string> = {
		industry: 'Choose Industry', card: 'Choose Card', build_location: 'Choose Location',
		road: 'Choose Road', second_road: 'Choose 2nd Road', coal_source: 'Coal Source',
		iron_source: 'Iron Source', beer_source: 'Beer Source', action_beer_source: 'Beer',
		sell_target: 'Sell Target', free_development: 'Free Develop', second_industry: 'Second Industry',
		network_mode: 'Network Mode', confirm: 'Confirm Action'
	};

	const MAP_KINDS = new Set(['build_location', 'road', 'second_road', 'sell_target', 'beer_source', 'action_beer_source']);
	const MODAL_KINDS = new Set(['industry', 'card', 'free_development', 'second_industry']);

	function onChoice(opt: any) {
		if (!cs) return;
		if (cs.kind === 'confirm') confirmAction();
		else applyChoice(cs.kind, opt.value);
	}

	async function copySeed() {
		if (!gs) return;
		try {
			await navigator.clipboard.writeText(String(gs.seed));
			seedCopied = true;
			if (seedCopyTimer) clearTimeout(seedCopyTimer);
			seedCopyTimer = setTimeout(() => {
				seedCopied = false;
				seedCopyTimer = null;
			}, 1300);
		} catch {
			// Non-critical if clipboard API unavailable.
		}
	}

	$: if (gs) {
		if (prevRound === -1) {
			prevRound = gs.round_in_phase;
			prevEra = gs.era;
		} else {
			const roundChanged = gs.round_in_phase !== prevRound;
			const eraChanged = gs.era !== prevEra;
			if (roundChanged || eraChanged) {
				if (toastTimer) clearTimeout(toastTimer);
				if (eraChanged && gs.era === 'Railroad') {
					roundToast = 'Railroad Era begins';
				} else if (roundChanged) {
					roundToast = `New Round ${gs.round_in_phase + 1}/${roundsPerEra(gs.players.length)}`;
				}
				toastTimer = setTimeout(() => {
					roundToast = null;
					toastTimer = null;
				}, 1800);
			}
			prevRound = gs.round_in_phase;
			prevEra = gs.era;
		}
	}
</script>

{#if gs}
<div class="sidebar">
	{#if roundToast}
		<div class="round-toast">{roundToast}</div>
	{/if}
	<section class="panel phase-panel">
		<div class="phase-era">{gs.era} Era</div>
		<div class="phase-sub">
			Turn {gs.turn_count + 1} &middot; Round {gs.round_in_phase + 1}/{roundsPerEra(gs.players.length)}
			&middot; Action {currentActionNumber()}/{actionBudget}
		</div>
		<button class="seed-chip" on:click={copySeed} title="Copy seed to clipboard">
			🎲 Seed {gs.seed}
		</button>
		{#if seedCopied}
			<div class="seed-copied">Copied seed</div>
		{/if}
	</section>

	<section class="panel">
		<h3>Players (Turn Order)</h3>
		{#each orderedPlayers as p, pos}
			<div class="player-row" class:active={p.index === gs.current_player}>
				<span class="turn-num">{pos + 1}</span>
				<span class="dot" style="background:{p.color}"></span>
				<span class="pname">{p.name}</span>
				<span class="pstats">
					£{p.money}<span class="money-delta">{moneyDelta(p.index, p.money)}</span>
					&middot; VP:{p.victory_points} &middot; Inc:{p.income_amount}
				</span>
				<button class="mini-btn" on:click={() => dispatch('openMat', { playerIndex: p.index })}>Mat</button>
				<button class="mini-btn" on:click={() => dispatch('openDiscard', { playerIndex: p.index })}>Discard</button>
			</div>
		{/each}
	</section>

	<section class="panel">
		<h3>Market</h3>
		<div class="market-row">
			<span class="cube coal"></span> Coal: {gs.coal_market}/14
			<span class="cube iron"></span> Iron: {gs.iron_market}/10
		</div>
	</section>

	<section class="panel controls">
		{#if phase === 'awaiting_start' && !gs.game_over}
			<button class="btn primary" on:click={startTurn}>Start Turn</button>
		{/if}
		{#if phase === 'turn_done' && !gs.game_over}
			<button class="btn secondary" on:click={endTurn}>End Turn</button>
		{/if}
		{#if gs.game_over}
			<div class="game-over">Game Over!</div>
		{/if}
		<button class="btn mat-btn" on:click={() => dispatch('openMat', { playerIndex: gs.current_player })}>My Industry Mat</button>
		<button class="btn mat-btn" on:click={() => dispatch('openDiscard', { playerIndex: gs.current_player })}>My Discard</button>
		{#if (phase === 'in_session' || phase === 'choosing_action' || phase === 'turn_done') && (gs.turn_action_history?.length ?? 0) > 0}
			<button class="btn undo-btn" on:click={undoLastAction}>Undo Previous Action</button>
		{/if}
	</section>

	{#if (gs.turn_action_history?.length ?? 0) > 0 || gs.current_action_selections}
	<section class="panel history-panel">
		<h3>Turn Progress</h3>
		{#each gs.turn_action_history ?? [] as act, idx}
			<div class="history-item">
				<div class="history-title">
					<span>{ACTION_EMOJI[act.action_type] || '🎯'}</span>
					<span>{idx + 1}. {ACTION_LABELS[act.action_type] || act.action_type}</span>
				</div>
				{#if act.selections.length > 0}
					<div class="history-tags">
						{#each act.selections as s}
							<span class="history-tag">{s}</span>
						{/each}
					</div>
				{/if}
			</div>
		{/each}
		{#if gs.current_action_selections}
			<div class="history-item current">
				<div class="history-title">
					<span>{ACTION_EMOJI[gs.current_action_selections.action_type] || '🎯'}</span>
					<span>Current: {ACTION_LABELS[gs.current_action_selections.action_type] || gs.current_action_selections.action_type}</span>
				</div>
				{#if gs.current_action_selections.selections.length > 0}
					<div class="history-tags">
						{#each gs.current_action_selections.selections as s}
							<span class="history-tag">{s}</span>
						{/each}
					</div>
				{:else}
					<div class="history-tags"><span class="history-tag">No selections yet</span></div>
				{/if}
			</div>
		{/if}
	</section>
	{/if}

	{#if phase === 'choosing_action' && actions}
	<section class="panel">
		<h3>Actions</h3>
		<div class="btn-grid">
			{#each actions as a}
				<button class="btn action" on:click={() => selectAction(a)}>
					{ACTION_LABELS[a] || a}
				</button>
			{/each}
		</div>
	</section>
	{/if}

	{#if phase === 'in_session' && cs && !MAP_KINDS.has(cs.kind) && !MODAL_KINDS.has(cs.kind)}
	<section class="panel choice-panel">
		<h3>{CHOICE_TITLES[cs.kind] || 'Choose'}</h3>
		<div class="btn-grid">
			{#each cs.options as opt}
				<button class="btn choice" on:click={() => onChoice(opt)}>
					{opt.label}
				</button>
			{/each}
		</div>
		<button class="btn cancel" on:click={cancelAction}>Cancel</button>
	</section>
	{/if}

	{#if phase === 'in_session' && cs && MAP_KINDS.has(cs.kind)}
	<section class="panel hint-panel">
		<h3>{CHOICE_TITLES[cs.kind] || 'Choose on map'}</h3>
		<p class="hint-text">Click a highlighted location on the board</p>
		<button class="btn cancel" on:click={cancelAction}>Cancel</button>
	</section>
	{/if}

	{#if phase === 'in_session' && cs && (cs.kind === 'industry' || cs.kind === 'free_development' || cs.kind === 'second_industry')}
	<section class="panel hint-panel develop-hint">
		<h3>{CHOICE_TITLES[cs.kind] || 'Choose'}</h3>
		{#if pendingDevs.length > 0}
			<div class="dev-selections">
				<span class="dev-label">Selected:</span>
				{#each pendingDevs as dev}
					<div class="dev-tile" style="--ind-color: {INDUSTRY_COLORS[dev.industry] ?? '#666'}">
						<img src={devIconPath(dev.industry)} alt={dev.industry} class="dev-tile-icon" />
						<span class="dev-tile-label">{dev.industry} {levelToRoman(dev.level)}</span>
					</div>
				{/each}
			</div>
		{/if}
		<p class="hint-text">Select from the Industry Mat</p>
		<button class="btn cancel" on:click={cancelAction}>Cancel</button>
	</section>
	{/if}

	{#if phase === 'in_session' && cs && cs.kind === 'card'}
	<section class="panel hint-panel">
		<h3>Choose Card</h3>
		<p class="hint-text">Click a card from your hand</p>
		<button class="btn cancel" on:click={cancelAction}>Cancel</button>
	</section>
	{/if}

	<section class="panel log-panel">
		<h3>Log</h3>
		<div class="log-scroll">
			{#each $logs as msg}
				<div class="log-entry">{msg}</div>
			{/each}
		</div>
	</section>
</div>
{/if}

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 8px;
		overflow-y: auto;
		height: 100vh;
		width: 290px;
		position: relative;
	}
	.round-toast {
		position: sticky;
		top: 0;
		z-index: 5;
		text-align: center;
		background: linear-gradient(90deg, #14532d, #166534);
		color: #dcfce7;
		border: 1px solid #22c55e;
		border-radius: 8px;
		padding: 8px 10px;
		font-size: 12px;
		font-weight: 700;
		animation: toast-in 180ms ease-out;
	}
	@keyframes toast-in {
		from { opacity: 0; transform: translateY(-6px); }
		to { opacity: 1; transform: translateY(0); }
	}
	.panel {
		background: #16213e;
		border-radius: 8px;
		padding: 10px 12px;
		border: 1px solid #2a3a5c;
	}
	.panel h3 {
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: #7a8ba8;
		margin: 0 0 6px;
	}
	.phase-panel { text-align: center; }
	.phase-era { font-size: 16px; font-weight: 700; color: #c7a750; }
	.phase-sub { font-size: 11px; color: #7a8ba8; margin-top: 2px; }
	.seed-chip {
		margin-top: 8px;
		background: #1e293b;
		border: 1px solid #334155;
		color: #cbd5e1;
		border-radius: 999px;
		padding: 4px 10px;
		font-size: 11px;
		font-weight: 700;
		cursor: pointer;
	}
	.seed-chip:hover { border-color: #22c55e; color: #dcfce7; }
	.seed-copied { margin-top: 4px; font-size: 10px; color: #86efac; }

	.player-row {
		display: flex; align-items: center; padding: 4px 6px; border-radius: 5px;
		margin-bottom: 2px; font-size: 12px;
	}
	.player-row.active { background: rgba(37,99,235,0.2); border: 1px solid #2563eb; }
	.turn-num {
		font-size: 10px; font-weight: 700; color: #64748b;
		width: 16px; text-align: center; flex-shrink: 0;
	}
	.dot { width: 10px; height: 10px; border-radius: 50%; margin-right: 6px; flex-shrink: 0; }
	.pname { font-weight: 600; min-width: 60px; }
	.pstats { color: #94a3b8; font-size: 11px; margin-left: auto; white-space: nowrap; }
	.money-delta { color: #ef4444; font-size: 10px; }
	.mini-btn {
		margin-left: 4px;
		padding: 2px 6px;
		font-size: 10px;
		border-radius: 6px;
		border: 1px solid #475569;
		background: #1e293b;
		color: #cbd5e1;
		cursor: pointer;
	}
	.mini-btn:hover {
		border-color: #7c3aed;
		color: #e2e8f0;
	}

	.market-row { display: flex; align-items: center; gap: 10px; font-size: 13px; }
	.cube { width: 12px; height: 12px; border-radius: 3px; display: inline-block; }
	.cube.coal { background: #1a1a1a; border: 1px solid #555; }
	.cube.iron { background: #f97316; }

	.controls { display: flex; justify-content: center; gap: 8px; flex-wrap: wrap; }
	.game-over { text-align: center; font-size: 18px; font-weight: 700; color: #ef4444; }

	.btn {
		padding: 7px 14px; border: none; border-radius: 6px; cursor: pointer;
		font-size: 12px; font-weight: 600; color: #fff; transition: background 0.15s;
	}
	.btn.primary { background: #2563eb; }
	.btn.primary:hover { background: #1d4ed8; }
	.btn.secondary { background: #475569; }
	.btn.secondary:hover { background: #374151; }
	.btn.action { background: #065f46; }
	.btn.action:hover { background: #047857; }
	.btn.choice { background: #7c3aed; min-width: 90px; }
	.btn.choice:hover { background: #6d28d9; }
	.btn.cancel { background: #991b1b; margin-top: 6px; }
	.btn.cancel:hover { background: #7f1d1d; }
	.btn.mat-btn { background: #334155; font-size: 11px; padding: 5px 10px; }
	.btn.mat-btn:hover { background: #475569; }
	.btn.undo-btn { background: #7f1d1d; font-size: 11px; padding: 5px 10px; }
	.btn.undo-btn:hover { background: #991b1b; }

	.btn-grid { display: flex; flex-wrap: wrap; gap: 4px; }

	.hint-panel { text-align: center; }
	.hint-text { color: #22c55e; font-size: 13px; margin: 8px 0; }
	.history-panel { display: flex; flex-direction: column; gap: 6px; }
	.history-item { background: rgba(15,23,42,0.6); border: 1px solid #2a3a5c; border-radius: 8px; padding: 6px; }
	.history-item.current { border-color: #22c55e; }
	.history-title { display: flex; align-items: center; gap: 6px; font-size: 12px; color: #e2e8f0; font-weight: 700; }
	.history-tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 6px; }
	.history-tag { font-size: 10px; color: #cbd5e1; background: #1e293b; border-radius: 999px; padding: 2px 8px; }
	.develop-hint .dev-selections {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		margin: 8px 0;
		justify-content: center;
	}
	.dev-label { font-size: 11px; color: #7a8ba8; margin-right: 4px; }
	.dev-tile {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		background: rgba(0,0,0,0.3);
		border-radius: 8px;
		border: 1px solid var(--ind-color, #555);
	}
	.dev-tile-icon { width: 24px; height: 24px; object-fit: contain; }
	.dev-tile-label { font-size: 12px; font-weight: 600; color: #eee; }

	.log-panel { flex: 1; min-height: 80px; }
	.log-scroll { max-height: 200px; overflow-y: auto; font-size: 10px; color: #94a3b8; line-height: 1.6; }
	.log-entry { border-bottom: 1px solid #1e293b; padding: 1px 0; }
</style>
