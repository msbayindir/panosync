# Demo

`docs/demo.gif` (shown at the top of the README) is generated from [`demo.tape`](demo.tape) with
[VHS](https://github.com/charmbracelet/vhs). It shows a local relay and two clients, "laptop" and
"desktop", each with its own clipboard and config directory, in one terminal split with tmux:

1. the relay starts;
2. the laptop runs `panosync setup` and gets a new key, the desktop is set up with that key;
3. text copied on the laptop is pasted on the desktop, then the other way round;
4. the laptop runs `panosync pause 5`, copies a "password", and it does not arrive on the desktop.

The logs show `sent (27 bytes)` / `received (27 bytes)`, never the text.

## Regenerating the GIF

Everything runs in Docker, so you only need Docker; nothing touches your own clipboard:

```bash
docker build -f docs/demo-env/Dockerfile -t panosync-demo .
docker run --rm -v "$PWD/docs:/vhs" panosync-demo
```

This records `demo.tape` to a video and converts it to `docs/demo.gif` ([render.sh](demo-env/render.sh)). The image ([docs/demo-env/](demo-env/)) builds panosync from the
current checkout and adds tmux, xclip and two virtual X displays (`Xvfb :1` and `:2`), one per
"computer". The shell helpers `laptop …` and `desktop …` run a command against one of them
(`DISPLAY` and `PANOSYNC_CONFIG_DIR`), and `copy` / `paste` use `xclip`.

To change the scenario, edit `demo.tape`; the layout is in `demo-env/start-demo.sh`. The key shown in
the GIF is a throwaway key generated during the recording.

## Adding a real recording with two computers

A recording of two real machines is more convincing than a terminal simulation. To add one later:

**Setup**

- Both machines visible in one recording: Ubuntu in a VM on the Mac (UTM, Parallels, VMware), or the
  Ubuntu machine through a VNC/RDP window, side by side with a Mac window.
- Both devices paired with a relay. Big terminal font (18–20 pt), Do Not Disturb on, no personal
  windows, and **never show the key** (`panosync key` or the `setup` output).
- On each side: a terminal running `panosync` (log visible) and a place to paste (a second terminal
  tab or a text editor).

**Scenario (15–20 seconds)**

1. Both screens visible, `panosync` running on both, log shows `connected to the relay`.
2. On the Mac, select `ssh deploy@10.0.0.5 -p 2222` in an editor and press <kbd>Cmd</kbd>+<kbd>C</kbd>;
   the logs show `sent (27 bytes)` / `received (27 bytes)`; paste on Ubuntu with
   <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>V</kbd>.
3. On Ubuntu, copy `Merhaba dünya, çğıöşü` and paste it on the Mac.
4. Optional: `panosync pause 1` on the Mac, copy something, show `paused, clipboard change not sent`.
5. Hold the last frame for a second.

**Recording and converting**

On macOS, <kbd>Cmd</kbd>+<kbd>Shift</kbd>+<kbd>5</kbd> → *Record Selected Portion* gives a `.mov` file.
Convert it with [gifski](https://gif.ski) (800 px wide, under ~5 MB):

```bash
brew install ffmpeg gifski
ffmpeg -i demo.mov -vf "fps=15,scale=800:-1:flags=lanczos" frames-%04d.png
gifski --fps 15 --width 800 --quality 80 -o docs/demo-real.gif frames-*.png
rm frames-*.png
```

Too big? Lower the frame rate to 10, the width to 720 or the quality to 70, or trim the ends
(`ffmpeg -ss 1 -to 19 -i demo.mov …`).

**Putting it in the README**

Save it as `docs/demo-real.gif` and either replace the image at the top of `README.md`:

```html
<p align="center"><img src="docs/demo-real.gif" alt="Copy on a Mac, paste on Ubuntu" width="800"></p>
```

or keep the VHS demo there and add the real recording below it under a short caption such as
*"The same on a real Mac and Ubuntu machine"*.
