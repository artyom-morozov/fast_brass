.PHONY: dev server client test build clean server-restart client-restart kill

# Run both server and client in a tmux session
dev:
	@./dev.sh

# Run just the Rust backend (port 3000)
server:
	@-lsof -ti tcp:3000 | xargs -r kill 2>/dev/null || true
	cargo run

# Run just the SvelteKit frontend (port 5173, proxies /api to :3000)
client:
	cd ui && npm run dev

# Build the Rust backend
build:
	cargo build

# Run all tests
test:
	cargo test

# Restart the server (kill existing and relaunch)
server-restart:
	@-lsof -ti tcp:3000 | xargs -r kill 2>/dev/null || true; sleep 1
	cargo run &

# Restart the client
client-restart:
	@-pkill -f 'vite dev' 2>/dev/null; sleep 1
	cd ui && npm run dev &

# Kill the tmux dev session
kill:
	@tmux kill-session -t brass-dev 2>/dev/null || echo "No session to kill"

# Type-check the frontend
check-ui:
	cd ui && npx svelte-check

# Build frontend for production
build-ui:
	cd ui && npm run build
