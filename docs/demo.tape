# VHS script for docs/demo.gif (https://github.com/charmbracelet/vhs).
# Runs inside the environment from docs/demo-env/ (see docs/demo.md), which also turns the
# video into docs/demo.gif:
#
#   docker build -f docs/demo-env/Dockerfile -t panosync-demo .
#   docker run --rm -v "$PWD/docs:/vhs" panosync-demo
#
# A local relay and two clients ("laptop" and "desktop", each with its own clipboard and config
# directory). Text copied on one appears on the other; the logs only show byte counts.

Output "/tmp/demo.mp4"

Require tmux
Require xclip
Require panosync
Require panosync-relay

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 680
Set Framerate 15
Set Padding 12
Set TypingSpeed 35ms
Set Theme "Catppuccin Mocha"

Hide
Type "start-demo"
Enter
Sleep 2s
Show

# 1. Start the relay.
Type "panosync-relay"
Sleep 400ms
Enter
Sleep 1.5s

# 2. Laptop: first device, generates a new key.
Ctrl+B
Down
Type "laptop panosync setup --server ws://localhost:8080"
Sleep 400ms
Enter
Sleep 3s
Type "clear; laptop panosync"
Enter
Sleep 1.5s

# 3. Desktop: second device, set up with the laptop's key.
Ctrl+B
Right
Type `desktop panosync setup --server ws://localhost:8080 --key "$(laptop panosync key)"`
Sleep 400ms
Enter
Sleep 2.5s
Type "clear; desktop panosync"
Enter
Sleep 2s

# 4. Copy on the laptop, paste on the desktop.
Ctrl+B
Down
Type "laptop copy 'ssh deploy@10.0.0.5 -p 2222'"
Sleep 400ms
Enter
Sleep 1.5s
Type "desktop paste"
Enter
Sleep 2.5s

# 5. And the other way round. (VHS cannot type non-ASCII text; Unicode is covered by the tests.)
Type "desktop copy 'Hello from the desktop'"
Sleep 400ms
Enter
Sleep 1.5s
Type "laptop paste"
Enter
Sleep 2.5s

# 6. Pause the laptop, copy a "password": it is not sent.
Type "laptop panosync pause 5"
Enter
Sleep 1.5s
Type "laptop copy 'hunter2'"
Enter
Sleep 1.5s
Type "desktop paste"
Enter
Sleep 4s
