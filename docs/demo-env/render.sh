#!/bin/bash
# Records docs/demo.tape to a video with VHS, then converts it to docs/demo.gif in two ffmpeg
# passes (palette first, then the GIF). VHS's own GIF output keeps every frame in memory, which
# fails in a Docker VM with little RAM.
set -euo pipefail
cd /vhs
vhs demo.tape
ffmpeg -loglevel error -y -i /tmp/demo.mp4 -vf "fps=12,scale=960:-1:flags=lanczos,palettegen=stats_mode=diff" /tmp/palette.png
ffmpeg -loglevel error -y -i /tmp/demo.mp4 -i /tmp/palette.png \
  -lavfi "fps=12,scale=960:-1:flags=lanczos[v];[v][1:v]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" \
  -loop 0 /vhs/demo.gif
ls -lh /vhs/demo.gif
