export interface PendingDevelopment {
	industry: string;
	level: number;
}

export interface TurnActionView {
	action_type: string;
	selections: string[];
}

export interface GameState {
	seed: number;
	current_player: number;
	phase: string;
	era: string;
	turn_count: number;
	round_in_phase: number;
	actions_remaining: number;
	players: Player[];
	buildings: Building[];
	roads: Road[];
	coal_market: number;
	iron_market: number;
	trade_posts: TradePostSlot[];
	available_actions: string[] | null;
	choice_set: ChoiceSet | null;
	pending_developments?: PendingDevelopment[];
	turn_action_history?: TurnActionView[];
	current_action_selections?: TurnActionView | null;
	discard_history?: DiscardEntry[];
	has_pending_shortfall: boolean;
	game_over: boolean;
	turn_order: number[];
}

export interface DiscardEntry {
	order: number;
	player_index: number;
	player_name: string;
	round_in_phase: number;
	turn_count: number;
	card_type: string;
	card_label: string;
}

export interface Player {
	index: number;
	name: string;
	color: string;
	money: number;
	income_level: number;
	income_amount: number;
	victory_points: number;
	hand: Card[];
	hand_size: number;
	industry_mat: IndustryTile[];
}

export interface IndustryTile {
	industry: string;
	level: number;
	tiles_remaining: number;
	money_cost: number;
	coal_cost: number;
	iron_cost: number;
	beer_needed: number;
	vp_on_flip: number;
	road_vp: number;
	resource_amt: number;
	income: number;
	removed_after_phase1: boolean;
	can_develop: boolean;
	exhausted: boolean;
}

export interface IndustryLevelData {
	industry: string;
	level: number;
	money_cost: number;
	coal_cost: number;
	iron_cost: number;
	beer_needed: number;
	vp_on_flip: number;
	road_vp: number;
	resource_amt: number;
	income: number;
	removed_after_phase1: boolean;
	can_develop: boolean;
	num_tiles: number;
}

export interface Card {
	index: number;
	card_type: string;
	label: string;
}

export interface Building {
	location: number;
	town: string;
	industry: string;
	level: number;
	owner: number;
	flipped: boolean;
	resource_amt: number;
	road_vp: number;
	vp_on_flip: number;
}

export interface Road {
	index: number;
	owner: number;
}

export interface TradePostSlot {
	slot_index: number;
	trade_post: string;
	tile_type: string | null;
	has_beer: boolean;
}

export interface ChoiceSet {
	kind: string;
	options: ChoiceOption[];
}

export interface ChoiceOption {
	value: any;
	label: string;
}
