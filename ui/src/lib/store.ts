import { writable, derived, get } from 'svelte/store';
import type { GameState, ChoiceSet, Player, IndustryLevelData } from './types';

export const gameState = writable<GameState | null>(null);
export type TurnPhase = 'awaiting_start' | 'choosing_action' | 'in_session' | 'turn_done';
export const turnPhase = writable<TurnPhase>('awaiting_start');
export const actionsAvailable = writable<string[] | null>(null);
export const logs = writable<string[]>([]);
export const allIndustryData = writable<Record<string, IndustryLevelData[]> | null>(null);

/** Money each player had at the start of the current player's turn */
export const moneyAtTurnStart = writable<Record<number, number>>({});
/** Currently selected action for this turn (for logging) */
export const currentAction = writable<string | null>(null);
export const actionBudgetAtTurnStart = writable<number>(1);

export function logMessage(msg: string) {
	logs.update(l => [msg, ...l].slice(0, 100));
}

export function snapshotMoney() {
	const gs = get(gameState);
	if (!gs) return;
	const snap: Record<number, number> = {};
	for (const p of gs.players) snap[p.index] = p.money;
	moneyAtTurnStart.set(snap);
}

export const currentPlayer = derived(gameState, $gs => {
	if (!$gs) return null;
	return $gs.players.find(p => p.index === $gs.current_player) ?? null;
});

export const choiceSet = derived(gameState, $gs => $gs?.choice_set ?? null);

/** Pending development choices (industry + level) during Develop/Develop x2 action */
export const pendingDevelopments = derived(
	gameState,
	$gs => $gs?.pending_developments ?? []
);

export function playerName(gs: GameState, idx: number): string {
	return gs.players.find(p => p.index === idx)?.name ?? `P${idx}`;
}