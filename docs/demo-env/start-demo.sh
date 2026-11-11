#!/bin/bash
# Starts two virtual X displays and the tmux layout used by docs/demo.tape:
#
#   ┌──────────────── relay ────────────────┐
#   ├───── laptop ──────┬───── desktop ─────┤
#   ├───────────── you (shell) ─────────────┤
set -e
Xvfb :1 -screen 0 320x240x24 >/dev/null 2>&1 &
Xvfb :2 -screen 0 320x240x24 >/dev/null 2>&1 &
rm -rf /demo && mkdir -p /demo
sleep 1

shell="bash --rcfile /etc/panosync-demo.bashrc"
tmux -f /etc/panosync-demo.tmux.conf new-session -d -s demo -x "$(tput cols)" -y "$(tput lines)" "$shell"
relay=$(tmux display-message -p -t demo '#{pane_id}')
laptop=$(tmux split-window -v -t "$relay" -l 78% -P -F '#{pane_id}' "$shell")
you=$(tmux split-window -v -t "$laptop" -l 40% -P -F '#{pane_id}' "$shell")
desktop=$(tmux split-window -h -t "$laptop" -P -F '#{pane_id}' "$shell")

tmux select-pane -t "$relay" -T "relay server"
tmux select-pane -t "$laptop" -T "laptop"
tmux select-pane -t "$desktop" -T "desktop"
tmux select-pane -t "$you" -T "you: copy on one computer, paste on the other"
# tmux moves to the most recently used pane in a direction: make "Down" from the relay go to the laptop.
tmux select-pane -t "$laptop"
tmux select-pane -t "$relay"
exec tmux attach -t demo
