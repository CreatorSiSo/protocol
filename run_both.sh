#!/bin/bash
tmux new-session -d -s mysession
tmux split-window -h
tmux send-keys -t mysession:0.0 "cd adapter-nano && cargo r -q -r" Enter
tmux send-keys -t mysession:0.1 "cd adapter-b15f && cargo r -q -r" Enter
tmux attach -t mysession
