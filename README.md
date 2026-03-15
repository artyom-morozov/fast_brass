# fast_brass

`fast_brass` is an unofficial Rust implementation of **Brass: Birmingham** board game focused on deterministic game logic, rule validation, browser-based playtestin and optional Python bindings for AI / RL workflows.

The repo contains the core board-game engine, axum HTTP server, a lightweight web UI for interactive play, and a test suite for various game-states.

## What This Repo Contains

- A Rust game engine for the main Brass: Birmingham action system
- Validation logic for legal builds, links, sales, development, loans, scouts, and passes
- A local web server and browser client for manual play and debugging
- SQLite-backed persistence for saved games
- Optional Python bindings via `PyO3` / `maturin`
- Integration and regression tests for gameplay edge cases


![Board UI view](docs/screenshots/board-overview.png)


## Quick Start

### Run the Rust backend

```bash
cargo run
```

The backend serves on `http://localhost:3000` by default.

### Run the full local playtest stack

```bash
make dev
```

This starts:

- the Rust backend on port `3000`
- the Svelte frontend in `ui/` on port `5173`

You can also run them separately:

```bash
make server
make client
```

### Run tests

```bash
cargo test
```

### Build Python bindings

```bash
maturin develop
```

This exposes the `fast_brass` Python module when the `python-bindings` feature is enabled through `maturin`.

## Repo Layout

- `src/core/` - basic game types, locations, static data, player mats
- `src/board/` - board state, resources, connectivity, board-level operations
- `src/actions/` - action execution logic
- `src/validation/` - legal move generation and rule validation
- `src/game/` - high-level action/session flow and turn runner
- `src/web/` - HTTP API and state serialization
- `src/python/` - optional Python API for RL / simulation workflows
- `tests/` - integration and regression tests
- `ui/` - browser UI used for manual play and debugging

## Brass: Birmingham Rules In Brief

Brass: Birmingham is a two-era economic network game about building industries and transport links in the English Midlands.

### Objective

Score the most victory points by the end of the Rail era. Points come mainly from:

- flipped industry tiles
- links connected to valuable developed locations
- strong positioning across both eras

### Turn structure

- The game is played across the **Canal** era and the **Rail** era.
- In the first round of the Canal era, each player takes **1 action**.
- In all later rounds, each player takes **2 actions** on their turn.
- **Every action requires discarding a card**, including `Pass`.

### Main action types

- `Build`: place an industry tile, pay its cost, and consume required resources
- `Network`: build canal or rail links to expand your network
- `Develop`: remove lower-level tiles from your mat to reach stronger industries
- `Sell`: flip Cotton, Goods, or Pottery by connecting to merchants and paying any beer cost
- `Loan`: gain money in exchange for dropping income
- `Scout`: trade extra cards for wild cards
- `Pass`: skip an action, but still discard a card

### Important rules to remember

- Building usually depends on both your network and the card you discard.
- Coal generally requires a connected source; iron does not.
- Beer is critical for selling and for some rail-era network actions.
- Level 1 tiles are removed from the board at the end of the Canal era.
- Turn order for the next round depends on how much money each player spent this round.

### Full rules

For the complete official rules, see the [Brass: Birmingham rulebook](https://cdn.1j1ju.com/medias/60/39/64-brass-birmingham-rulebook.pdf).

## Project Notes

- This is an **unofficial** implementation intended for engine development, testing, AI experimentation, and browser play.
- The focus of this repo is correctness and debuggability of the game state.
- The public API and module structure may continue to evolve as the engine is split into a dedicated standalone repository.
