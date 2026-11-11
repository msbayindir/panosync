# Shell helpers for the demo. Each "computer" has its own X display (its own clipboard)
# and its own panosync config directory.
laptop()  { DISPLAY=:1 PANOSYNC_CONFIG_DIR=/demo/laptop  "$@"; }
desktop() { DISPLAY=:2 PANOSYNC_CONFIG_DIR=/demo/desktop "$@"; }
# Copy text to / paste text from the clipboard of the display above.
copy()  { printf '%s' "$1" | xclip -selection clipboard; }
paste() { xclip -selection clipboard -o; echo; }
PS1='\[\e[1;32m\]$\[\e[0m\] '
