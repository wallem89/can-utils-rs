#!/usr/bin/env bash
set -euo pipefail

tmux kill-session -t can-demo 2>/dev/null || true
tmux new-session -d -s can-demo

# Left pane: dump flow
tmux send-keys -t can-demo:0.0 'cargo run' Enter

# Right pane: send flow
tmux split-window -h -t can-demo:0.0
tmux send-keys -t can-demo:0.1 'cargo run' Enter

tmux select-layout -t can-demo even-horizontal

exec tmux attach -t can-demo