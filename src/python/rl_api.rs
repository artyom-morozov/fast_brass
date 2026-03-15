use std::collections::HashSet;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use rand::seq::SliceRandom;

use crate::actions::{BuildOption, SellOption, SingleRailroadOption};
use crate::board::resources::{BeerSellSource, BreweryBeerSource, ResourceSource};
use crate::consts::{
    MAX_MARKET_COAL, MAX_MARKET_IRON, N_ROAD_LOCATIONS, N_BL, N_LOCATIONS, N_PLAYERS, NUM_TRADE_POSTS,
    TWO_RAILROAD_PRICE,
};
use crate::core::locations::LocationName;
use crate::core::static_data::{BUILD_LOCATION_MASK, INDUSTRY_MAT, LINK_LOCATIONS};
use crate::core::types::{ActionType, BitSetWrapper, Card, CardType, Era, IndustryType};
use crate::game::framework::{ActionChoice, ChoiceSet, NetworkMode, ShortfallResolutionSession};
use crate::game::runner::{GamePhase, GameRunner};
use crate::market::merchants::MerchantTileType;

const ROOT_ACTION_COUNT: usize = 7;
const CARD_TYPE_DIM: usize = 29;
const MAX_HAND_MASK_DIM: usize = 64;
const NETWORK_MODE_DIM: usize = 2;
const COAL_SOURCE_DIM: usize = N_BL + 2; // build locations + market + stop token
const IRON_SOURCE_DIM: usize = N_BL + 2; // build locations + market + stop token
const BEER_SOURCE_DIM: usize = N_BL + (NUM_TRADE_POSTS * 2) + 1; // breweries + merchant slots + stop
const ACTION_BEER_SOURCE_DIM: usize = (N_BL * 2) + 1; // own + opponent breweries + stop
const SELL_TARGET_DIM: usize = N_BL + 1; // build locations + stop
const SHORTFALL_TILE_DIM: usize = N_BL + 1; // build locations + stop

const ROOT_BUILD_BUILDING: usize = 0;
const ROOT_BUILD_RAILROAD: usize = 1;
const ROOT_DEVELOP: usize = 2;
const ROOT_DEVELOP_DOUBLE: usize = 3;
const ROOT_SELL: usize = 4;
const ROOT_LOAN: usize = 5;
const ROOT_SCOUT: usize = 6;

const STOP_INDEX_COAL: usize = COAL_SOURCE_DIM - 1;
const STOP_INDEX_IRON: usize = IRON_SOURCE_DIM - 1;
const STOP_INDEX_BEER: usize = BEER_SOURCE_DIM - 1;
const STOP_INDEX_ACTION_BEER: usize = ACTION_BEER_SOURCE_DIM - 1;
const STOP_INDEX_SELL_TARGET: usize = SELL_TARGET_DIM - 1;
const STOP_INDEX_SHORTFALL: usize = SHORTFALL_TILE_DIM - 1;

#[derive(Clone, Default)]
struct CompositeActionPayload {
    root_action: Option<usize>,
    card_values: Vec<usize>,
    industry_values: Vec<usize>,
    second_industry_values: Vec<usize>,
    build_location_values: Vec<usize>,
    network_mode_values: Vec<usize>,
    road_values: Vec<usize>,
    second_road_values: Vec<usize>,
    coal_source_values: Vec<usize>,
    iron_source_values: Vec<usize>,
    beer_source_values: Vec<usize>,
    action_beer_source_values: Vec<usize>,
    sell_target_values: Vec<usize>,
    shortfall_tile_order: Vec<usize>,

    card_idx: usize,
    industry_idx: usize,
    second_industry_idx: usize,
    build_location_idx: usize,
    network_mode_idx: usize,
    road_idx: usize,
    second_road_idx: usize,
    coal_source_idx: usize,
    iron_source_idx: usize,
    beer_source_idx: usize,
    action_beer_source_idx: usize,
    sell_target_idx: usize,
    used_card_choices: HashSet<usize>,
}

impl CompositeActionPayload {
    fn from_py(action: &Bound<'_, PyDict>) -> PyResult<Self> {
        let root_action = extract_optional_usize(action, "root_action")?;
        Ok(Self {
            root_action,
            card_values: extract_usize_list(action, "card")?,
            industry_values: extract_usize_list(action, "industry")?,
            second_industry_values: extract_usize_list(action, "second_industry")?,
            build_location_values: extract_usize_list(action, "build_location")?,
            network_mode_values: extract_usize_list(action, "network_mode")?,
            road_values: extract_usize_list(action, "road")?,
            second_road_values: extract_usize_list(action, "second_road")?,
            coal_source_values: extract_usize_list(action, "coal_sources")?,
            iron_source_values: extract_usize_list(action, "iron_sources")?,
            beer_source_values: extract_usize_list(action, "beer_sources")?,
            action_beer_source_values: extract_usize_list(action, "action_beer_source")?,
            sell_target_values: extract_usize_list(action, "sell_targets")?,
            shortfall_tile_order: extract_usize_list(action, "shortfall_tile_order")?,
            card_idx: 0,
            industry_idx: 0,
            second_industry_idx: 0,
            build_location_idx: 0,
            network_mode_idx: 0,
            road_idx: 0,
            second_road_idx: 0,
            coal_source_idx: 0,
            iron_source_idx: 0,
            beer_source_idx: 0,
            action_beer_source_idx: 0,
            sell_target_idx: 0,
            used_card_choices: HashSet::new(),
        })
    }

    fn next_card(&mut self) -> Option<usize> {
        next_from_list(&self.card_values, &mut self.card_idx)
    }

    fn next_industry(&mut self) -> Option<usize> {
        next_from_list(&self.industry_values, &mut self.industry_idx)
    }

    fn next_second_industry(&mut self) -> Option<usize> {
        next_from_list(&self.second_industry_values, &mut self.second_industry_idx)
    }

    fn next_build_location(&mut self) -> Option<usize> {
        next_from_list(&self.build_location_values, &mut self.build_location_idx)
    }

    fn next_network_mode(&mut self) -> Option<usize> {
        next_from_list(&self.network_mode_values, &mut self.network_mode_idx)
    }

    fn next_road(&mut self) -> Option<usize> {
        next_from_list(&self.road_values, &mut self.road_idx)
    }

    fn next_second_road(&mut self) -> Option<usize> {
        next_from_list(&self.second_road_values, &mut self.second_road_idx)
    }

    fn next_coal_source(&mut self) -> Option<usize> {
        next_from_list(&self.coal_source_values, &mut self.coal_source_idx)
    }

    fn next_iron_source(&mut self) -> Option<usize> {
        next_from_list(&self.iron_source_values, &mut self.iron_source_idx)
    }

    fn next_beer_source(&mut self) -> Option<usize> {
        next_from_list(&self.beer_source_values, &mut self.beer_source_idx)
    }

    fn next_action_beer_source(&mut self) -> Option<usize> {
        next_from_list(
            &self.action_beer_source_values,
            &mut self.action_beer_source_idx,
        )
    }

    fn next_sell_target(&mut self) -> Option<usize> {
        next_from_list(&self.sell_target_values, &mut self.sell_target_idx)
    }

    fn select_unique_card_from_options(&mut self, options: &[usize]) -> PyResult<usize> {
        if let Some(requested_idx) = self.next_card() {
            if options.contains(&requested_idx) && !self.used_card_choices.contains(&requested_idx) {
                self.used_card_choices.insert(requested_idx);
                return Ok(requested_idx);
            }
        }

        if let Some(fallback) = options
            .iter()
            .copied()
            .find(|idx| !self.used_card_choices.contains(idx))
        {
            self.used_card_choices.insert(fallback);
            return Ok(fallback);
        }

        let Some(fallback) = options.first().copied() else {
            return Err(PyValueError::new_err("ChoiceSet::Card had no options"));
        };
        self.used_card_choices.insert(fallback);
        Ok(fallback)
    }
}

#[pyclass(unsendable)]
#[derive(Clone)]
pub struct BrassSavedState {
    runner: GameRunner,
    current_shortfall: Option<ShortfallResolutionSession>,
}

#[pyclass(unsendable)]
pub struct BrassRLGame {
    runner: GameRunner,
    current_shortfall: Option<ShortfallResolutionSession>,
}

#[pymethods]
impl BrassRLGame {
    #[new]
    #[pyo3(signature = (num_players=4, seed=None))]
    fn new(num_players: usize, seed: Option<u64>) -> PyResult<Self> {
        if !(2..=4).contains(&num_players) {
            return Err(PyValueError::new_err(
                "num_players must be in [2, 4] for Brass Birmingham",
            ));
        }
        let mut game = Self {
            runner: GameRunner::new(num_players, seed),
            current_shortfall: None,
        };
        let _ = game.advance_to_next_decision()?;
        Ok(game)
    }

    #[pyo3(signature = (seed=None, num_players=None))]
    fn reset(&mut self, seed: Option<u64>, num_players: Option<usize>) -> PyResult<()> {
        let next_players = num_players.unwrap_or(self.runner.framework.board.state.players.len());
        if !(2..=4).contains(&next_players) {
            return Err(PyValueError::new_err(
                "num_players must be in [2, 4] for Brass Birmingham",
            ));
        }
        self.runner = GameRunner::new(next_players, seed);
        self.current_shortfall = None;
        let _ = self.advance_to_next_decision()?;
        Ok(())
    }

    fn current_decision_player(&self) -> usize {
        self.current_shortfall
            .as_ref()
            .map(|s| s.player_idx)
            .unwrap_or(self.runner.framework.current_player)
    }

    fn current_decision_mode(&self) -> String {
        if self.current_shortfall.is_some() {
            "shortfall".to_string()
        } else {
            "turn".to_string()
        }
    }

    fn num_players(&self) -> usize {
        self.runner.framework.board.state.players.len()
    }

    fn is_done(&self) -> bool {
        self.runner.is_game_finished()
    }

    fn get_observation(&mut self, py: Python<'_>, observer_idx: usize) -> PyResult<PyObject> {
        self.validate_player_index(observer_idx)?;
        let out = PyDict::new(py);
        self.write_observation(py, observer_idx, &out)?;
        Ok(out.into())
    }

    fn get_action_masks(&mut self, py: Python<'_>, observer_idx: usize) -> PyResult<PyObject> {
        self.validate_player_index(observer_idx)?;
        let out = PyDict::new(py);
        self.write_action_masks(py, observer_idx, &out)?;
        Ok(out.into())
    }

    fn get_spaces(&self, py: Python<'_>) -> PyResult<PyObject> {
        let out = PyDict::new(py);
        out.set_item("root_action_dim", ROOT_ACTION_COUNT)?;
        out.set_item("card_dim", MAX_HAND_MASK_DIM)?;
        out.set_item("industry_dim", 6)?;
        out.set_item("build_location_dim", N_BL)?;
        out.set_item("road_dim", N_ROAD_LOCATIONS)?;
        out.set_item("network_mode_dim", NETWORK_MODE_DIM)?;
        out.set_item("coal_source_dim", COAL_SOURCE_DIM)?;
        out.set_item("iron_source_dim", IRON_SOURCE_DIM)?;
        out.set_item("beer_source_dim", BEER_SOURCE_DIM)?;
        out.set_item("action_beer_source_dim", ACTION_BEER_SOURCE_DIM)?;
        out.set_item("sell_target_dim", SELL_TARGET_DIM)?;
        out.set_item("shortfall_tile_dim", SHORTFALL_TILE_DIM)?;
        Ok(out.into())
    }

    fn available_root_actions(&mut self) -> PyResult<Vec<usize>> {
        let _ = self.advance_to_next_decision()?;
        Ok(self.filtered_root_actions()
            .into_iter()
            .filter_map(root_action_to_index)
            .collect())
    }

    fn save_state(&self) -> BrassSavedState {
        BrassSavedState {
            runner: self.runner.clone(),
            current_shortfall: self.current_shortfall.clone(),
        }
    }

    fn restore_state(&mut self, state: &BrassSavedState) -> PyResult<()> {
        self.runner = state.runner.clone();
        self.current_shortfall = state.current_shortfall.clone();
        let _ = self.advance_to_next_decision()?;
        Ok(())
    }

    fn randomize_hidden_information(&mut self, observer_idx: usize) -> PyResult<()> {
        self.validate_player_index(observer_idx)?;
        if self.runner.is_game_finished() {
            return Ok(());
        }

        let state = &mut self.runner.framework.board.state;
        let mut hidden_cards: Vec<Card> = state.deck.cards.clone();

        for (player_idx, player) in state.players.iter().enumerate() {
            if player_idx != observer_idx {
                hidden_cards.extend(player.hand.cards.iter().cloned());
            }
        }

        hidden_cards.shuffle(&mut state.rng);

        let mut cursor = 0usize;
        for player_idx in 0..state.players.len() {
            if player_idx == observer_idx {
                continue;
            }
            let hand_len = state.players[player_idx].hand.cards.len();
            let end = cursor + hand_len;
            if end > hidden_cards.len() {
                return Err(PyValueError::new_err(
                    "Failed to randomize hidden information: inconsistent hidden card pool",
                ));
            }
            state.players[player_idx].hand.cards = hidden_cards[cursor..end].to_vec();
            cursor = end;
        }

        state.deck.cards = hidden_cards[cursor..].to_vec();
        Ok(())
    }

    fn step_composite_action(
        &mut self,
        py: Python<'_>,
        action: &Bound<'_, PyDict>,
    ) -> PyResult<PyObject> {
        let mut forced_passes = self.advance_to_next_decision()?;
        let decision_mode_before = self.current_decision_mode();
        let acting_player = self.current_decision_player();
        let phase_before = self.runner.game_phase;
        let vps_before = current_vps(&self.runner);
        let potential_vps_before = current_potential_vps(&self.runner);
        let shortfall_before = self
            .current_shortfall
            .as_ref()
            .map(|s| (s.player_idx, s.shortfall))
            .unwrap_or((usize::MAX, 0));
        let mut sold_buildings_count: usize = 0;
        let mut liquidation_tile_count: usize = 0;
        let mut action_type_name = "shortfall".to_string();
        let mut action_type_id: i32 = -1;

        if self.runner.is_game_finished() {
            let out = self.build_step_delta(
                py,
                acting_player,
                &decision_mode_before,
                &action_type_name,
                action_type_id,
                forced_passes,
                sold_buildings_count,
                liquidation_tile_count,
                phase_before,
                vps_before,
                potential_vps_before,
                shortfall_before,
            )?;
            return Ok(out);
        }

        if let Some(session) = self.current_shortfall.take() {
            let payload = CompositeActionPayload::from_py(action)?;
            let chosen_tiles = payload.shortfall_tile_order;
            liquidation_tile_count = chosen_tiles.len();
            self.runner.resolve_shortfall_with_tiles(session, chosen_tiles);
        } else {
            let base_payload = CompositeActionPayload::from_py(action)?;
            let valid_roots = self.filtered_root_actions();
            if valid_roots.is_empty() {
                return Err(PyValueError::new_err(
                    "No legal non-PASS root action available for turn decision",
                ));
            }

            let desired_root = base_payload
                .root_action
                .and_then(index_to_root_action)
                .filter(|a| valid_roots.contains(a))
                .unwrap_or(valid_roots[0]);

            let mut candidate_roots = vec![desired_root];
            candidate_roots.extend(valid_roots.iter().copied().filter(|root| *root != desired_root));

            let mut executed_root: Option<ActionType> = None;
            let mut last_error_message: Option<String> = None;

            for candidate_root in candidate_roots {
                let mut payload = base_payload.clone();
                let _ = self.runner.start_action(candidate_root);

                let mut choice_loop_guard = 0usize;
                let mut selection_failed = false;
                loop {
                    choice_loop_guard += 1;
                    if choice_loop_guard > 512 {
                        return Err(PyValueError::new_err(
                            "Action session choice loop exceeded safety limit",
                        ));
                    }
                    let Some(choice_set) = self.runner.framework.get_next_choice_set() else {
                        break;
                    };
                    if matches!(choice_set, ChoiceSet::ConfirmOnly) {
                        break;
                    }
                    let choice = match select_choice_from_payload(&choice_set, &mut payload) {
                        Ok(choice) => choice,
                        Err(err) => {
                            last_error_message = Some(err.to_string());
                            selection_failed = true;
                            break;
                        }
                    };
                    if let Err(err) = self.runner.framework.apply_action_choice(choice) {
                        last_error_message =
                            Some(format!("apply_action_choice failed: {}", err));
                        selection_failed = true;
                        break;
                    }
                }

                if selection_failed {
                    self.runner.framework.cancel_action_session();
                    continue;
                }

                if let Some(session) = self.runner.framework.current_session() {
                    if session.action_type == ActionType::Sell {
                        sold_buildings_count = session.intent.sell_choices.len();
                    }
                }

                match self.runner.confirm_action() {
                    Ok(()) => {
                        executed_root = Some(candidate_root);
                        break;
                    }
                    Err(err) => {
                        last_error_message = Some(format!("confirm_action failed: {}", err));
                        self.runner.framework.cancel_action_session();
                    }
                }
            }

            let Some(executed_root) = executed_root else {
                let reason =
                    last_error_message.unwrap_or_else(|| "unknown action-session failure".to_string());
                return Err(PyValueError::new_err(format!(
                    "Failed to execute any legal root action: {}",
                    reason
                )));
            };

            action_type_name = root_action_name(executed_root).to_string();
            action_type_id = root_action_to_index(executed_root)
                .map(|idx| idx as i32)
                .unwrap_or(-1);

            if self.runner.actions_remaining_in_turn == 0 {
                self.runner.end_turn();
            }
        }

        forced_passes += self.advance_to_next_decision()?;
        let out = self.build_step_delta(
            py,
            acting_player,
            &decision_mode_before,
            &action_type_name,
            action_type_id,
            forced_passes,
            sold_buildings_count,
            liquidation_tile_count,
            phase_before,
            vps_before,
            potential_vps_before,
            shortfall_before,
        )?;
        Ok(out)
    }
}

impl BrassRLGame {
    fn validate_player_index(&self, player_idx: usize) -> PyResult<()> {
        if player_idx >= self.runner.framework.board.state.players.len() {
            return Err(PyValueError::new_err(format!(
                "player index {} out of bounds for {} players",
                player_idx,
                self.runner.framework.board.state.players.len()
            )));
        }
        Ok(())
    }

    fn pull_shortfall_if_needed(&mut self) {
        if self.current_shortfall.is_none() && !self.runner.pending_shortfall_sessions.is_empty() {
            self.current_shortfall = Some(self.runner.pending_shortfall_sessions.remove(0));
        }
    }

    fn filtered_root_actions(&self) -> Vec<ActionType> {
        self.runner
            .framework
            .get_valid_root_actions()
            .into_iter()
            .filter(|a| *a != ActionType::Pass && *a != ActionType::BuildDoubleRailroad)
            .collect()
    }

    fn advance_to_next_decision(&mut self) -> PyResult<usize> {
        let mut forced_passes = 0usize;
        let mut guard = 0usize;

        while guard < 256 {
            guard += 1;
            if self.runner.is_game_finished() {
                break;
            }

            self.pull_shortfall_if_needed();
            if self.current_shortfall.is_some() {
                break;
            }

            if self.runner.actions_remaining_in_turn == 0 {
                let _ = self.runner.start_turn();
            }

            let roots = self.filtered_root_actions();
            if !roots.is_empty() {
                break;
            }

            let valid_actions = self.runner.framework.get_valid_root_actions();
            if valid_actions.contains(&ActionType::Pass) {
                let _ = self.runner.start_action(ActionType::Pass);
                let mut auto_payload = CompositeActionPayload::default();
                let mut choice_loop_guard = 0usize;
                loop {
                    choice_loop_guard += 1;
                    if choice_loop_guard > 128 {
                        return Err(PyValueError::new_err(
                            "auto-PASS choice loop exceeded safety limit",
                        ));
                    }
                    let Some(choice_set) = self.runner.framework.get_next_choice_set() else {
                        break;
                    };
                    if matches!(choice_set, ChoiceSet::ConfirmOnly) {
                        break;
                    }
                    let choice = select_choice_from_payload(&choice_set, &mut auto_payload)?;
                    self.runner
                        .framework
                        .apply_action_choice(choice)
                        .map_err(|e| {
                            PyValueError::new_err(format!(
                                "auto-PASS apply_action_choice failed: {}",
                                e
                            ))
                        })?;
                }
                self.runner.confirm_action().map_err(|e| {
                    PyValueError::new_err(format!("auto-PASS confirm failed: {}", e))
                })?;
                forced_passes += 1;
            } else {
                // Safety fallback: if no legal action and no pass, burn the action slot.
                self.runner.end_action_slot();
            }

            if self.runner.actions_remaining_in_turn == 0 {
                self.runner.end_turn();
            }
        }

        if guard >= 256 {
            return Err(PyValueError::new_err(
                "advance_to_next_decision exceeded safety limit",
            ));
        }
        Ok(forced_passes)
    }

    fn write_observation(
        &self,
        _py: Python<'_>,
        observer_idx: usize,
        out: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        let state = &self.runner.framework.board.state;
        let num_players = state.players.len();
        let decision_player = self
            .current_shortfall
            .as_ref()
            .map(|s| s.player_idx)
            .unwrap_or(self.runner.framework.current_player);
        let decision_mode = if self.current_shortfall.is_some() {
            "shortfall"
        } else {
            "turn"
        };

        let mut phase_one_hot = vec![0.0f32; 3];
        phase_one_hot[match self.runner.game_phase {
            GamePhase::Canal => 0,
            GamePhase::Railroad => 1,
            GamePhase::GameEnd => 2,
        }] = 1.0;
        let mut era_one_hot = vec![0.0f32; 2];
        era_one_hot[match state.era {
            Era::Canal => 0,
            Era::Railroad => 1,
        }] = 1.0;

        let global_features = vec![
            phase_one_hot[0],
            phase_one_hot[1],
            phase_one_hot[2],
            era_one_hot[0],
            era_one_hot[1],
            self.runner.turn_count as f32 / 200.0,
            self.runner.round_in_phase as f32 / 16.0,
            self.runner.actions_remaining_in_turn as f32 / 2.0,
            state.wild_location_cards_available as f32 / 4.0,
            state.wild_industry_cards_available as f32 / 4.0,
            state.remaining_market_coal as f32 / MAX_MARKET_COAL as f32,
            state.remaining_market_iron as f32 / MAX_MARKET_IRON as f32,
            if self.current_shortfall.is_some() { 1.0 } else { 0.0 },
            if self.runner.has_pending_shortfall() { 1.0 } else { 0.0 },
            observer_idx as f32 / (num_players.max(1) as f32),
            decision_player as f32 / (num_players.max(1) as f32),
        ];

        out.set_item("decision_mode", decision_mode)?;
        out.set_item("decision_player", decision_player)?;
        out.set_item("observer_idx", observer_idx)?;
        out.set_item("global_features", global_features)?;
        out.set_item(
            "turn_order",
            state
                .turn_order
                .iter()
                .map(|idx| *idx as i64)
                .collect::<Vec<i64>>(),
        )?;
        out.set_item("actions_remaining", self.runner.actions_remaining_in_turn)?;
        out.set_item("turn_count", self.runner.turn_count)?;
        out.set_item("round_in_phase", self.runner.round_in_phase)?;

        let mut discard_counts = vec![0.0f32; CARD_TYPE_DIM];
        for card in &state.discard_pile {
            let idx = card_type_to_index(card);
            discard_counts[idx] += 1.0;
        }
        out.set_item("discard_counts", discard_counts)?;

        let mut trade_post_slots = Vec::<i64>::new();
        let mut trade_post_beer = Vec::<f32>::new();
        for slot_idx in 0..(NUM_TRADE_POSTS * 2) {
            let slot_val = state
                .trade_post_slots
                .get(slot_idx)
                .and_then(|t| t.as_ref())
                .map(|tile| merchant_tile_type_to_index(&tile.tile_type))
                .unwrap_or(-1);
            trade_post_slots.push(slot_val);
            trade_post_beer.push(if state.trade_post_beer.contains(slot_idx) {
                1.0
            } else {
                0.0
            });
        }
        out.set_item("trade_post_slots", trade_post_slots)?;
        out.set_item("trade_post_beer", trade_post_beer)?;

        let mut buildings = Vec::<Vec<f32>>::with_capacity(N_BL);
        for bl_idx in 0..N_BL {
            let mut row = vec![0.0f32; 1 + N_PLAYERS + 6 + 1 + 1 + 1 + 6];
            if let Some(building) = state.bl_to_building.get(&bl_idx) {
                row[0] = 1.0;
                let owner = building.owner.as_usize().min(N_PLAYERS - 1);
                row[1 + owner] = 1.0;
                row[1 + N_PLAYERS + building.industry as usize] = 1.0;
                row[1 + N_PLAYERS + 6] = (building.level.as_usize() as f32 + 1.0) / 8.0;
                row[1 + N_PLAYERS + 6 + 1] = building.resource_amt as f32 / 5.0;
                row[1 + N_PLAYERS + 6 + 1 + 1] = if building.flipped { 1.0 } else { 0.0 };
            }
            for ind_idx in 0..6 {
                row[1 + N_PLAYERS + 6 + 1 + 1 + 1 + ind_idx] =
                    if BUILD_LOCATION_MASK[bl_idx].contains(ind_idx) {
                        1.0
                    } else {
                        0.0
                    };
            }
            buildings.push(row);
        }
        out.set_item("buildings", buildings)?;

        let mut roads = Vec::<Vec<f32>>::with_capacity(N_ROAD_LOCATIONS);
        for road_idx in 0..N_ROAD_LOCATIONS {
            let mut row = vec![0.0f32; 1 + N_PLAYERS + 2 + 3];
            row[1 + N_PLAYERS] = if LINK_LOCATIONS[road_idx].can_build_canal {
                1.0
            } else {
                0.0
            };
            row[1 + N_PLAYERS + 1] = if LINK_LOCATIONS[road_idx].can_build_rail {
                1.0
            } else {
                0.0
            };
            if state.built_roads.contains(road_idx) {
                row[0] = 1.0;
                for p_idx in 0..num_players {
                    if state.player_road_mask[p_idx].contains(road_idx) {
                        row[1 + p_idx.min(N_PLAYERS - 1)] = 1.0;
                        break;
                    }
                }
            }
            let mut loc_values = [-1.0f32; 3];
            for (k, loc_idx) in LINK_LOCATIONS[road_idx].locations.ones().enumerate().take(3) {
                loc_values[k] = loc_idx as f32 / (N_LOCATIONS as f32);
            }
            row[1 + N_PLAYERS + 2] = loc_values[0];
            row[1 + N_PLAYERS + 2 + 1] = loc_values[1];
            row[1 + N_PLAYERS + 2 + 2] = loc_values[2];
            roads.push(row);
        }
        out.set_item("roads", roads)?;

        let mut players_public = Vec::<Vec<f32>>::with_capacity(num_players);
        let mut industry_mats = Vec::<Vec<f32>>::with_capacity(num_players);
        let mut hand_sizes = Vec::<f32>::with_capacity(num_players);
        let mut player_build_masks = Vec::<Vec<f32>>::with_capacity(num_players);
        let mut player_road_masks = Vec::<Vec<f32>>::with_capacity(num_players);

        for p_idx in 0..num_players {
            let player = &state.players[p_idx];
            players_public.push(vec![
                if p_idx == observer_idx { 1.0 } else { 0.0 },
                if p_idx == self.runner.framework.current_player {
                    1.0
                } else {
                    0.0
                },
                if p_idx == decision_player { 1.0 } else { 0.0 },
                player.money as f32 / 100.0,
                player.income_level as f32 / 100.0,
                player.get_income_amount(player.income_level) as f32 / 30.0,
                player.victory_points as f32 / 100.0,
                state.visible_vps[p_idx] as f32 / 100.0,
                player.hand.cards.len() as f32 / MAX_HAND_MASK_DIM as f32,
                player.spent_this_turn as f32 / 100.0,
            ]);

            hand_sizes.push(player.hand.cards.len() as f32 / MAX_HAND_MASK_DIM as f32);

            let mut mat_row = Vec::<f32>::with_capacity(6 * 3);
            for ind_idx in 0..6 {
                let industry = IndustryType::from_usize(ind_idx);
                let level = player.industry_mat.get_lowest_level(industry);
                let remaining = player.industry_mat.get_remaining_tiles_at_level(industry);
                mat_row.push((level.as_usize() as f32 + 1.0) / 8.0);
                mat_row.push(remaining as f32 / 3.0);
                mat_row.push(if player.industry_mat.has_tiles_left(industry) {
                    1.0
                } else {
                    0.0
                });
            }
            industry_mats.push(mat_row);

            let mut building_mask = vec![0.0f32; N_BL];
            for bl_idx in state.player_building_mask[p_idx].ones() {
                if bl_idx < N_BL {
                    building_mask[bl_idx] = 1.0;
                }
            }
            player_build_masks.push(building_mask);

            let mut road_mask = vec![0.0f32; N_ROAD_LOCATIONS];
            for road_idx in state.player_road_mask[p_idx].ones() {
                if road_idx < N_ROAD_LOCATIONS {
                    road_mask[road_idx] = 1.0;
                }
            }
            player_road_masks.push(road_mask);
        }

        out.set_item("players_public", players_public)?;
        out.set_item("industry_mats", industry_mats)?;
        out.set_item("hand_sizes", hand_sizes)?;
        out.set_item("player_building_masks", player_build_masks)?;
        out.set_item("player_road_masks", player_road_masks)?;

        let mut self_hand_counts = vec![0.0f32; CARD_TYPE_DIM];
        for card in &state.players[observer_idx].hand.cards {
            let idx = card_type_to_index(card);
            self_hand_counts[idx] += 1.0;
        }
        out.set_item("self_hand_counts", self_hand_counts)?;

        if let Some(shortfall) = &self.current_shortfall {
            let sf = PyDict::new(out.py());
            sf.set_item("player_idx", shortfall.player_idx)?;
            sf.set_item("shortfall", shortfall.shortfall)?;
            let tiles: Vec<(usize, u16)> = shortfall
                .removable_tiles
                .iter()
                .map(|t| (t.build_location_idx, t.liquidation_value))
                .collect();
            sf.set_item("removable_tiles", tiles)?;
            out.set_item("shortfall_state", sf)?;
        } else {
            out.set_item("shortfall_state", out.py().None())?;
        }
        Ok(())
    }

    fn write_action_masks(
        &mut self,
        _py: Python<'_>,
        observer_idx: usize,
        out: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        self.validate_player_index(observer_idx)?;

        let mut root_mask = vec![0.0f32; ROOT_ACTION_COUNT];
        let mut card_mask = vec![0.0f32; MAX_HAND_MASK_DIM];
        let mut industry_mask = vec![0.0f32; 6];
        let mut second_industry_mask = vec![0.0f32; 6];
        let mut build_location_mask = vec![0.0f32; N_BL];
        let mut network_mode_mask = vec![0.0f32; NETWORK_MODE_DIM];
        let mut road_mask = vec![0.0f32; N_ROAD_LOCATIONS];
        let mut second_road_mask = vec![0.0f32; N_ROAD_LOCATIONS];
        let mut coal_source_mask = vec![0.0f32; COAL_SOURCE_DIM];
        let mut iron_source_mask = vec![0.0f32; IRON_SOURCE_DIM];
        let mut beer_source_mask = vec![0.0f32; BEER_SOURCE_DIM];
        let mut action_beer_source_mask = vec![0.0f32; ACTION_BEER_SOURCE_DIM];
        let mut sell_target_mask = vec![0.0f32; SELL_TARGET_DIM];
        let mut shortfall_tile_mask = vec![0.0f32; SHORTFALL_TILE_DIM];

        coal_source_mask[STOP_INDEX_COAL] = 1.0;
        iron_source_mask[STOP_INDEX_IRON] = 1.0;
        beer_source_mask[STOP_INDEX_BEER] = 1.0;
        action_beer_source_mask[STOP_INDEX_ACTION_BEER] = 1.0;
        sell_target_mask[STOP_INDEX_SELL_TARGET] = 1.0;
        shortfall_tile_mask[STOP_INDEX_SHORTFALL] = 1.0;

        if let Some(shortfall) = &self.current_shortfall {
            for tile in &shortfall.removable_tiles {
                if tile.build_location_idx < N_BL {
                    shortfall_tile_mask[tile.build_location_idx] = 1.0;
                }
            }
            out.set_item("decision_mode", "shortfall")?;
        } else {
            out.set_item("decision_mode", "turn")?;
            for action in self.filtered_root_actions() {
                if let Some(idx) = root_action_to_index(action) {
                    root_mask[idx] = 1.0;
                }
            }

            let state = &self.runner.framework.board.state;
            let player_idx = self.runner.framework.current_player;
            let hand_len = state.players[player_idx].hand.cards.len().min(MAX_HAND_MASK_DIM);
            for idx in 0..hand_len {
                card_mask[idx] = 1.0;
            }

            let validation = self.runner.framework.compute_valid_options();
            if let Some(build_opts) = validation.build_options.as_ref() {
                write_build_masks(
                    build_opts,
                    &mut industry_mask,
                    &mut build_location_mask,
                    &mut coal_source_mask,
                    &mut iron_source_mask,
                );
            }

            if let Some(dev_opts) = validation.dev_options.as_ref() {
                for ind_idx in dev_opts.ones() {
                    if ind_idx < 6 {
                        industry_mask[ind_idx] = 1.0;
                        second_industry_mask[ind_idx] = 1.0;
                    }
                }
            }

            let free_dev = self
                .runner
                .framework
                .board
                .get_valid_free_development_options(player_idx);
            for ind_idx in free_dev.ones() {
                if ind_idx < 6 {
                    second_industry_mask[ind_idx] = 1.0;
                }
            }

            let iron_sources_for_one = self
                .runner
                .framework
                .board
                .get_iron_sources_for_develop(player_idx, 1);
            for src in iron_sources_for_one {
                mark_resource_source(&mut iron_source_mask, src, STOP_INDEX_IRON);
            }
            let iron_sources_for_two = self
                .runner
                .framework
                .board
                .get_iron_sources_for_develop(player_idx, 2);
            for src in iron_sources_for_two {
                mark_resource_source(&mut iron_source_mask, src, STOP_INDEX_IRON);
            }

            if let Some(canal_opts) = validation.canal_options.as_ref() {
                if !canal_opts.is_empty() {
                    network_mode_mask[0] = 1.0;
                }
                for road_idx in canal_opts {
                    if *road_idx < N_ROAD_LOCATIONS {
                        road_mask[*road_idx] = 1.0;
                    }
                }
            }

            if let Some(single_opts) = validation.single_rail_options.as_ref() {
                if !single_opts.is_empty() {
                    network_mode_mask[0] = 1.0;
                }
                write_single_rail_masks(single_opts, &mut road_mask, &mut coal_source_mask);
            }

            if let Some(double_opts) = validation.double_rail_first_link_options.as_ref() {
                if !double_opts.is_empty() {
                    network_mode_mask[1] = 1.0;
                }
                write_single_rail_masks(double_opts, &mut road_mask, &mut coal_source_mask);
                write_double_rail_followup_masks(
                    &self.runner,
                    player_idx,
                    double_opts,
                    &mut second_road_mask,
                    &mut coal_source_mask,
                    &mut action_beer_source_mask,
                );
            }

            if let Some(sell_opts) = validation.sell_options.as_ref() {
                write_sell_masks(sell_opts, &mut sell_target_mask, &mut beer_source_mask);
            }
        }

        out.set_item("root_action_mask", root_mask)?;
        out.set_item("card_mask", card_mask)?;
        out.set_item("industry_mask", industry_mask)?;
        out.set_item("second_industry_mask", second_industry_mask)?;
        out.set_item("build_location_mask", build_location_mask)?;
        out.set_item("network_mode_mask", network_mode_mask)?;
        out.set_item("road_mask", road_mask)?;
        out.set_item("second_road_mask", second_road_mask)?;
        out.set_item("coal_source_mask", coal_source_mask)?;
        out.set_item("iron_source_mask", iron_source_mask)?;
        out.set_item("beer_source_mask", beer_source_mask)?;
        out.set_item("action_beer_source_mask", action_beer_source_mask)?;
        out.set_item("sell_target_mask", sell_target_mask)?;
        out.set_item("shortfall_tile_mask", shortfall_tile_mask)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn build_step_delta(
        &mut self,
        py: Python<'_>,
        acting_player: usize,
        decision_mode_before: &str,
        action_type_name: &str,
        action_type_id: i32,
        forced_passes: usize,
        sold_buildings_count: usize,
        liquidation_tile_count: usize,
        phase_before: GamePhase,
        vps_before: Vec<u16>,
        potential_vps_before: Vec<u16>,
        shortfall_before: (usize, u16),
    ) -> PyResult<PyObject> {
        let done = self.runner.is_game_finished();
        let vps_after = current_vps(&self.runner);
        let vp_delta: Vec<i32> = vps_after
            .iter()
            .zip(vps_before.iter())
            .map(|(after, before)| *after as i32 - *before as i32)
            .collect();
        let potential_vps_after = current_potential_vps(&self.runner);
        let potential_vp_delta: Vec<i32> = potential_vps_after
            .iter()
            .zip(potential_vps_before.iter())
            .map(|(after, before)| *after as i32 - *before as i32)
            .collect();
        let phase_transition = phase_before != self.runner.game_phase;

        let mut entered_shortfall = false;
        if let Some(sf) = &self.current_shortfall {
            if sf.player_idx == acting_player && shortfall_before.0 != acting_player {
                entered_shortfall = true;
            }
        } else if self
            .runner
            .pending_shortfall_sessions
            .iter()
            .any(|s| s.player_idx == acting_player)
            && shortfall_before.0 != acting_player
        {
            entered_shortfall = true;
        }

        let winner = if done {
            self.runner
                .framework
                .board
                .state
                .players
                .iter()
                .enumerate()
                .max_by_key(|(_, p)| p.victory_points)
                .map(|(idx, _)| idx as i64)
        } else {
            None
        };

        let mut next_root_mask = vec![0.0f32; ROOT_ACTION_COUNT];
        for action in self.filtered_root_actions() {
            if let Some(idx) = root_action_to_index(action) {
                next_root_mask[idx] = 1.0;
            }
        }

        let out = PyDict::new(py);
        out.set_item("acting_player", acting_player)?;
        out.set_item("decision_mode_before", decision_mode_before)?;
        out.set_item("action_type", action_type_name)?;
        out.set_item("action_type_id", action_type_id)?;
        out.set_item("forced_passes", forced_passes)?;
        out.set_item("sold_buildings", sold_buildings_count)?;
        out.set_item("liquidation_tiles", liquidation_tile_count)?;
        out.set_item("entered_shortfall", entered_shortfall)?;
        out.set_item("phase_transition", phase_transition)?;
        out.set_item("vp_delta", vp_delta)?;
        out.set_item("potential_vp_delta", potential_vp_delta)?;
        out.set_item("vps", vps_after)?;
        out.set_item("potential_vps", potential_vps_after)?;
        out.set_item("done", done)?;
        out.set_item("winner", winner)?;
        out.set_item("next_decision_player", self.current_decision_player())?;
        out.set_item("next_decision_mode", self.current_decision_mode())?;
        out.set_item("next_root_action_mask", next_root_mask)?;
        out.set_item("round_in_phase", self.runner.round_in_phase)?;
        out.set_item("turn_count", self.runner.turn_count)?;
        Ok(out.into())
    }
}

fn next_from_list(values: &[usize], idx: &mut usize) -> Option<usize> {
    if *idx < values.len() {
        let value = values[*idx];
        *idx += 1;
        Some(value)
    } else {
        None
    }
}

fn extract_optional_usize(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<usize>> {
    let maybe = dict.get_item(key)?;
    let Some(value) = maybe else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    Ok(Some(value.extract::<usize>()?))
}

fn extract_usize_list(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Vec<usize>> {
    let maybe = dict.get_item(key)?;
    let Some(value) = maybe else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }

    if let Ok(list) = value.downcast::<PyList>() {
        return list
            .iter()
            .map(|item| item.extract::<usize>())
            .collect::<PyResult<Vec<usize>>>();
    }
    if let Ok(tuple) = value.downcast::<PyTuple>() {
        return tuple
            .iter()
            .map(|item| item.extract::<usize>())
            .collect::<PyResult<Vec<usize>>>();
    }
    if let Ok(single) = value.extract::<usize>() {
        return Ok(vec![single]);
    }
    Err(PyValueError::new_err(format!(
        "Value for '{}' must be an int or list of ints",
        key
    )))
}

fn root_action_to_index(action: ActionType) -> Option<usize> {
    match action {
        ActionType::BuildBuilding => Some(ROOT_BUILD_BUILDING),
        ActionType::BuildRailroad => Some(ROOT_BUILD_RAILROAD),
        ActionType::Develop => Some(ROOT_DEVELOP),
        ActionType::DevelopDouble => Some(ROOT_DEVELOP_DOUBLE),
        ActionType::Sell => Some(ROOT_SELL),
        ActionType::Loan => Some(ROOT_LOAN),
        ActionType::Scout => Some(ROOT_SCOUT),
        _ => None,
    }
}

fn index_to_root_action(index: usize) -> Option<ActionType> {
    match index {
        ROOT_BUILD_BUILDING => Some(ActionType::BuildBuilding),
        ROOT_BUILD_RAILROAD => Some(ActionType::BuildRailroad),
        ROOT_DEVELOP => Some(ActionType::Develop),
        ROOT_DEVELOP_DOUBLE => Some(ActionType::DevelopDouble),
        ROOT_SELL => Some(ActionType::Sell),
        ROOT_LOAN => Some(ActionType::Loan),
        ROOT_SCOUT => Some(ActionType::Scout),
        _ => None,
    }
}

fn root_action_name(action: ActionType) -> &'static str {
    match action {
        ActionType::BuildBuilding => "build_building",
        ActionType::BuildRailroad => "build_railroad",
        ActionType::Develop => "develop",
        ActionType::DevelopDouble => "develop_double",
        ActionType::Sell => "sell",
        ActionType::Loan => "loan",
        ActionType::Scout => "scout",
        ActionType::Pass => "pass",
        ActionType::BuildDoubleRailroad => "build_double_railroad",
    }
}

fn current_vps(runner: &GameRunner) -> Vec<u16> {
    runner
        .framework
        .board
        .state
        .players
        .iter()
        .map(|p| p.victory_points)
        .collect()
}

fn current_potential_vps(runner: &GameRunner) -> Vec<u16> {
    let state = &runner.framework.board.state;
    let num_players = state.players.len();
    let mut potential_vps = vec![0u16; num_players];

    for road_idx in state.built_roads.ones() {
        let mut road_owner = None;
        for p_idx in 0..num_players {
            if state.player_road_mask[p_idx].contains(road_idx) {
                road_owner = Some(p_idx);
                break;
            }
        }

        if let Some(owner_idx) = road_owner {
            let mut road_vps: u16 = 0;
            for loc_idx in LINK_LOCATIONS[road_idx].locations.ones() {
                let bl_set = LocationName::from_usize(loc_idx).to_bl_set();
                for bl_idx in bl_set.ones() {
                    if let Some(building) = state.bl_to_building.get(&bl_idx) {
                        let data = &INDUSTRY_MAT[building.industry as usize][building.level.as_usize()];
                        road_vps = road_vps.saturating_add(data.road_vp as u16);
                    }
                }
            }
            potential_vps[owner_idx] = potential_vps[owner_idx].saturating_add(road_vps);
        }
    }

    for building in state.bl_to_building.values() {
        if building.flipped {
            let data = &INDUSTRY_MAT[building.industry as usize][building.level.as_usize()];
            let owner_idx = building.owner.as_usize();
            if owner_idx < potential_vps.len() {
                potential_vps[owner_idx] =
                    potential_vps[owner_idx].saturating_add(data.vp_on_flip as u16);
            }
        }
    }

    potential_vps
}

fn card_type_to_index(card: &Card) -> usize {
    match &card.card_type {
        CardType::Location(town) => town.as_usize(),
        CardType::Industry(industry_set) => {
            let inds = industry_set.to_industry_types();
            if inds.len() == 2
                && inds.contains(&IndustryType::Cotton)
                && inds.contains(&IndustryType::Goods)
            {
                26
            } else if inds.len() == 1 {
                20 + inds[0].as_usize()
            } else {
                26
            }
        }
        CardType::WildLocation => 27,
        CardType::WildIndustry => 28,
    }
}

fn merchant_tile_type_to_index(tile: &MerchantTileType) -> i64 {
    match tile {
        MerchantTileType::All => 0,
        MerchantTileType::Cotton => 1,
        MerchantTileType::Goods => 2,
        MerchantTileType::Pottery => 3,
        MerchantTileType::Blank => 4,
    }
}

fn mark_resource_source(mask: &mut [f32], source: ResourceSource, stop_idx: usize) {
    let idx = match source {
        ResourceSource::Building(loc) => loc.min(stop_idx.saturating_sub(1)),
        ResourceSource::Market => N_BL,
    };
    if idx < stop_idx {
        mask[idx] = 1.0;
    }
}

fn write_build_masks(
    build_opts: &[BuildOption],
    industry_mask: &mut [f32],
    build_location_mask: &mut [f32],
    coal_source_mask: &mut [f32],
    iron_source_mask: &mut [f32],
) {
    for opt in build_opts {
        let ind_idx = opt.industry_type.as_usize();
        if ind_idx < industry_mask.len() {
            industry_mask[ind_idx] = 1.0;
        }
        if opt.build_location_idx < build_location_mask.len() {
            build_location_mask[opt.build_location_idx] = 1.0;
        }
        for src in &opt.coal_sources {
            mark_resource_source(coal_source_mask, *src, STOP_INDEX_COAL);
        }
        for src in &opt.iron_sources {
            mark_resource_source(iron_source_mask, *src, STOP_INDEX_IRON);
        }
    }
}

fn write_single_rail_masks(
    rail_opts: &[SingleRailroadOption],
    road_mask: &mut [f32],
    coal_source_mask: &mut [f32],
) {
    for option in rail_opts {
        if option.road_idx < road_mask.len() {
            road_mask[option.road_idx] = 1.0;
        }
        for src_idx in option.potential_coal_sources.ones() {
            let src = if src_idx >= N_BL {
                ResourceSource::Market
            } else {
                ResourceSource::Building(src_idx)
            };
            mark_resource_source(coal_source_mask, src, STOP_INDEX_COAL);
        }
    }
}

fn write_double_rail_followup_masks(
    runner: &GameRunner,
    player_idx: usize,
    first_link_opts: &[SingleRailroadOption],
    second_road_mask: &mut [f32],
    coal_source_mask: &mut [f32],
    action_beer_source_mask: &mut [f32],
) {
    let mut seen_second_roads = HashSet::<usize>::new();
    let mut seen_beer = HashSet::<usize>::new();
    let hypothetical_money_remaining = runner.framework.board.state.players[player_idx]
        .money
        .saturating_sub(TWO_RAILROAD_PRICE);

    for first_link in first_link_opts {
        for source_idx in first_link.potential_coal_sources.ones() {
            let coal_source = if source_idx >= N_BL {
                ResourceSource::Market
            } else {
                ResourceSource::Building(source_idx)
            };
            let followups = runner.framework.board.get_options_for_second_rail_link(
                player_idx,
                first_link.road_idx,
                &coal_source,
                hypothetical_money_remaining,
            );
            for followup in followups {
                if seen_second_roads.insert(followup.second_road_idx)
                    && followup.second_road_idx < second_road_mask.len()
                {
                    second_road_mask[followup.second_road_idx] = 1.0;
                }
                for src in followup.potential_coal_sources_for_second_link {
                    mark_resource_source(coal_source_mask, src, STOP_INDEX_COAL);
                }
                for src in followup.potential_beer_sources_for_action {
                    let idx = encode_action_beer_source(src);
                    if seen_beer.insert(idx) && idx < STOP_INDEX_ACTION_BEER {
                        action_beer_source_mask[idx] = 1.0;
                    }
                }
                for src in followup.own_brewery_sources {
                    let idx = encode_action_beer_source(src);
                    if seen_beer.insert(idx) && idx < STOP_INDEX_ACTION_BEER {
                        action_beer_source_mask[idx] = 1.0;
                    }
                }
            }
        }
    }
}

fn write_sell_masks(
    sell_opts: &[SellOption],
    sell_target_mask: &mut [f32],
    beer_source_mask: &mut [f32],
) {
    for sell_opt in sell_opts {
        if sell_opt.location < STOP_INDEX_SELL_TARGET {
            sell_target_mask[sell_opt.location] = 1.0;
        }
        for src_idx in sell_opt.beer_locations.ones() {
            if src_idx < STOP_INDEX_BEER {
                beer_source_mask[src_idx] = 1.0;
            }
        }
    }
}

fn select_choice_from_payload(
    choice_set: &ChoiceSet,
    payload: &mut CompositeActionPayload,
) -> PyResult<ActionChoice> {
    match choice_set {
        ChoiceSet::Industry(options) => {
            let desired = payload
                .next_industry()
                .and_then(|idx| options.iter().copied().find(|ind| ind.as_usize() == idx))
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::Industry had no options"))?;
            Ok(ActionChoice::Industry(desired))
        }
        ChoiceSet::SecondIndustry(options) => {
            let desired = payload
                .next_second_industry()
                .and_then(|idx| options.iter().copied().find(|ind| ind.as_usize() == idx))
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::SecondIndustry had no options"))?;
            Ok(ActionChoice::FreeDevelopment(desired))
        }
        ChoiceSet::Card(options) => {
            let desired = payload.select_unique_card_from_options(options)?;
            Ok(ActionChoice::Card(desired))
        }
        ChoiceSet::BuildLocation(options) => {
            let desired = payload
                .next_build_location()
                .and_then(|idx| options.iter().copied().find(|loc_idx| *loc_idx == idx))
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::BuildLocation had no options"))?;
            Ok(ActionChoice::BuildLocation(desired))
        }
        ChoiceSet::Road(options) => {
            let desired = payload
                .next_road()
                .and_then(|idx| options.iter().copied().find(|road_idx| *road_idx == idx))
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::Road had no options"))?;
            Ok(ActionChoice::Road(desired))
        }
        ChoiceSet::SecondRoad(options) => {
            let desired = payload
                .next_second_road()
                .and_then(|idx| options.iter().copied().find(|road_idx| *road_idx == idx))
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::SecondRoad had no options"))?;
            Ok(ActionChoice::Road(desired))
        }
        ChoiceSet::CoalSource(options) => {
            let desired = payload
                .next_coal_source()
                .and_then(|encoded| {
                    options
                        .iter()
                        .copied()
                        .find(|src| encode_resource_source(*src) == encoded)
                })
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::CoalSource had no options"))?;
            Ok(ActionChoice::CoalSource(desired))
        }
        ChoiceSet::IronSource(options) => {
            let desired = payload
                .next_iron_source()
                .and_then(|encoded| {
                    options
                        .iter()
                        .copied()
                        .find(|src| encode_resource_source(*src) == encoded)
                })
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::IronSource had no options"))?;
            Ok(ActionChoice::IronSource(desired))
        }
        ChoiceSet::BeerSource(options) => {
            let desired = payload
                .next_beer_source()
                .and_then(|encoded| {
                    options
                        .iter()
                        .copied()
                        .find(|src| encode_beer_sell_source(*src) == encoded)
                })
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::BeerSource had no options"))?;
            Ok(ActionChoice::BeerSource(desired))
        }
        ChoiceSet::ActionBeerSource(options) => {
            let desired = payload
                .next_action_beer_source()
                .and_then(|encoded| {
                    options
                        .iter()
                        .copied()
                        .find(|src| encode_action_beer_source(*src) == encoded)
                })
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::ActionBeerSource had no options"))?;
            Ok(ActionChoice::ActionBeerSource(desired))
        }
        ChoiceSet::SellTarget(options) => {
            let desired = payload
                .next_sell_target()
                .and_then(|idx| options.iter().copied().find(|loc_idx| *loc_idx == idx))
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::SellTarget had no options"))?;
            Ok(ActionChoice::SellTarget(desired))
        }
        ChoiceSet::FreeDevelopment(options) => {
            let desired = payload
                .next_second_industry()
                .and_then(|idx| options.iter().copied().find(|ind| ind.as_usize() == idx))
                .or_else(|| {
                    payload
                        .next_industry()
                        .and_then(|idx| options.iter().copied().find(|ind| ind.as_usize() == idx))
                })
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::FreeDevelopment had no options"))?;
            Ok(ActionChoice::FreeDevelopment(desired))
        }
        ChoiceSet::NetworkMode(options) => {
            let desired = payload
                .next_network_mode()
                .and_then(|idx| {
                    options.iter().copied().find(|mode| match (idx, mode) {
                        (0, NetworkMode::Single) => true,
                        (1, NetworkMode::Double) => true,
                        _ => false,
                    })
                })
                .or_else(|| options.first().copied())
                .ok_or_else(|| PyValueError::new_err("ChoiceSet::NetworkMode had no options"))?;
            Ok(ActionChoice::NetworkMode(desired))
        }
        ChoiceSet::ConfirmOnly => Ok(ActionChoice::Confirm),
    }
}

fn encode_resource_source(source: ResourceSource) -> usize {
    match source {
        ResourceSource::Building(loc) => loc,
        ResourceSource::Market => N_BL,
    }
}

fn encode_beer_sell_source(source: BeerSellSource) -> usize {
    match source {
        BeerSellSource::Building(loc) => loc,
        BeerSellSource::TradePost(slot) => N_BL + slot,
    }
}

fn encode_action_beer_source(source: BreweryBeerSource) -> usize {
    match source {
        BreweryBeerSource::OwnBrewery(loc) => loc,
        BreweryBeerSource::OpponentBrewery(loc) => N_BL + loc,
    }
}

#[pymodule]
pub fn fast_brass(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<BrassRLGame>()?;
    m.add_class::<BrassSavedState>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::building::BuiltBuilding;
    use crate::core::player::PlayerId;
    use crate::core::types::IndustryLevel;

    #[test]
    fn test_bridge_observation_and_masks_smoke() {
        Python::with_gil(|py| {
            let mut game = BrassRLGame::new(2, Some(7)).expect("bridge init should work");
            let observer = game.current_decision_player();

            let obs_obj = game
                .get_observation(py, observer)
                .expect("observation call should work");
            let obs = obs_obj
                .bind(py)
                .downcast::<PyDict>()
                .expect("obs should be dict");
            assert!(obs.contains("global_features").unwrap());
            assert!(obs.contains("buildings").unwrap());
            assert!(obs.contains("self_hand_counts").unwrap());

            let masks_obj = game
                .get_action_masks(py, observer)
                .expect("masks call should work");
            let masks = masks_obj
                .bind(py)
                .downcast::<PyDict>()
                .expect("masks should be dict");
            assert!(masks.contains("root_action_mask").unwrap());
            assert!(masks.contains("shortfall_tile_mask").unwrap());
        });
    }

    #[test]
    fn test_save_restore_round_trip() {
        Python::with_gil(|py| {
            let mut game = BrassRLGame::new(2, Some(11)).expect("bridge init should work");
            let saved = game.save_state();
            let before_player = game.current_decision_player();
            let before_mode = game.current_decision_mode();

            let roots = game.available_root_actions().expect("roots should be available");
            let action = PyDict::new(py);
            action.set_item("root_action", roots[0]).unwrap();
            let _ = game
                .step_composite_action(py, &action)
                .expect("single step should work");

            game.restore_state(&saved).expect("restore should work");
            assert_eq!(before_player, game.current_decision_player());
            assert_eq!(before_mode, game.current_decision_mode());
        });
    }

    #[test]
    fn test_forced_pass_auto_progress() {
        let mut game = BrassRLGame::new(2, Some(13)).expect("bridge init should work");
        let player_idx = game.runner.framework.current_player;

        // Make all non-pass actions unavailable while keeping one card for PASS.
        game.runner.framework.board.state.players[player_idx].money = 0;
        game.runner.framework.board.state.players[player_idx].income_level = 0; // income -10 => loan unavailable
        game.runner.framework.board.state.players[player_idx]
            .hand
            .cards
            .truncate(1);
        game.runner.framework.board.state.players[player_idx]
            .industry_mat = crate::core::industry_mat::PlayerIndustryMat::new();
        game.runner.framework.board.state.wild_location_cards_available = 0;
        game.runner.framework.board.state.wild_industry_cards_available = 0;
        game.runner.framework.board.state.bl_to_building.clear();
        game.runner.framework.board.state.player_building_mask[player_idx].clear();
        game.runner.framework.board.state.build_locations_occupied.clear();
        game.runner.framework.board.state.built_roads.clear();
        game.runner.framework.board.state.player_road_mask[player_idx].clear();
        game.runner.actions_remaining_in_turn = 1;

        let forced = game
            .advance_to_next_decision()
            .expect("auto-progress should not fail");
        assert!(forced >= 1, "expected at least one forced pass");
    }

    #[test]
    fn test_shortfall_liquidation_event_fields() {
        Python::with_gil(|py| {
            let mut game = BrassRLGame::new(2, Some(21)).expect("bridge init should work");
            let player_idx = game.runner.framework.current_player;
            let loc = 27usize;

            let building = BuiltBuilding::build(
                IndustryType::Coal,
                IndustryLevel::I,
                loc as u8,
                PlayerId::from_usize(player_idx),
            );
            game.runner
                .framework
                .board
                .state
                .bl_to_building
                .insert(loc, building);
            game.runner
                .framework
                .board
                .state
                .build_locations_occupied
                .insert(loc);
            game.runner.framework.board.state.player_building_mask[player_idx].insert(loc);
            game.runner.framework.board.state.players[player_idx].money = 0;

            let session = game
                .runner
                .framework
                .start_shortfall_resolution_session(player_idx, 3);
            game.current_shortfall = Some(session);

            let action = PyDict::new(py);
            action.set_item("shortfall_tile_order", vec![loc]).unwrap();
            let delta_obj = game
                .step_composite_action(py, &action)
                .expect("shortfall step should work");
            let delta = delta_obj
                .bind(py)
                .downcast::<PyDict>()
                .expect("delta should be dict");
            let liquidation_tiles: usize = delta
                .get_item("liquidation_tiles")
                .unwrap()
                .unwrap()
                .extract()
                .unwrap();
            assert!(liquidation_tiles >= 1);
        });
    }

    #[test]
    fn test_hidden_information_randomization_preserves_public_constraints() {
        let mut game = BrassRLGame::new(4, Some(31)).expect("bridge init should work");
        let observer = 0usize;

        let observer_hand_before = game.runner.framework.board.state.players[observer]
            .hand
            .cards
            .clone();
        let opp_sizes_before: Vec<usize> = game
            .runner
            .framework
            .board
            .state
            .players
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != observer)
            .map(|(_, p)| p.hand.cards.len())
            .collect();

        let mut hidden_counts_before = vec![0usize; CARD_TYPE_DIM];
        for (idx, player) in game.runner.framework.board.state.players.iter().enumerate() {
            if idx != observer {
                for card in &player.hand.cards {
                    hidden_counts_before[card_type_to_index(card)] += 1;
                }
            }
        }
        for card in &game.runner.framework.board.state.deck.cards {
            hidden_counts_before[card_type_to_index(card)] += 1;
        }

        game.randomize_hidden_information(observer)
            .expect("hidden info randomization should succeed");

        assert_eq!(
            observer_hand_before,
            game.runner.framework.board.state.players[observer].hand.cards
        );
        let opp_sizes_after: Vec<usize> = game
            .runner
            .framework
            .board
            .state
            .players
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != observer)
            .map(|(_, p)| p.hand.cards.len())
            .collect();
        assert_eq!(opp_sizes_before, opp_sizes_after);

        let mut hidden_counts_after = vec![0usize; CARD_TYPE_DIM];
        for (idx, player) in game.runner.framework.board.state.players.iter().enumerate() {
            if idx != observer {
                for card in &player.hand.cards {
                    hidden_counts_after[card_type_to_index(card)] += 1;
                }
            }
        }
        for card in &game.runner.framework.board.state.deck.cards {
            hidden_counts_after[card_type_to_index(card)] += 1;
        }
        assert_eq!(hidden_counts_before, hidden_counts_after);
    }
}
