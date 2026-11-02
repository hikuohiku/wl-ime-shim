# wl-ime-shim

A tool that scales the position of IME candidate windows on Wayland.

In Chromium and Electron apps with a scaled UI, the candidate window appears up
and to the left of the caret, and the error grows with the distance from the
window's top-left corner.

![A chat app with text being composed in the message box. Without the proxy, the app reports the caret at the midpoint of the line from the window's top-left corner to the real caret, so the candidate window appears there. With the proxy, it appears directly above the caret.](docs/ime-position.svg)

Chromium can scale its UI on top of the output scale, for example from
`gtk-xft-dpi` or `--force-device-scale-factor`, but does not apply this scale to
the caret rectangle it passes to `set_cursor_rectangle`. This tool runs a program
behind a small Wayland proxy and multiplies that rectangle by a factor you choose.

> [!WARNING]
> [Chromium CL 8599053](https://chromium-review.googlesource.com/c/chromium/src/+/8599053)
> fixes this for font-based UI scaling, starting with Chromium 158. Once an app
> includes the fix, this proxy scales the rectangle a second time, so stop using
> it for that app.

## Usage

```sh
wl-ime-shim --scale 2 discord
```

Everything after the program name is passed to the program, so put this tool's
options before it:

```sh
wl-ime-shim --scale 2 discord --ozone-platform=wayland --enable-wayland-ime
```

`--scale` is the factor applied to the rectangle. If the candidate window appears
at half the caret's distance from the window's top-left corner, use `2`.

Use `--log` to print each rectangle while you type:

```console
$ wl-ime-shim --scale 2 --log discord
zwp_text_input_v3.set_cursor_rectangle (164, 52, 0, 10) -> (328, 104, 0, 20)
```

If the app is already running and only allows one instance, quit it first.
Otherwise the new process hands over to the running one, which is not behind
the proxy.

> [!NOTE]
> - The compositor sees the proxy as the client, so a window's PID is the proxy's.
> - The app is not offered protocols that the bundled `wl-proxy` does not know
>   about. On niri, this hides `wl_eglstream_display` and `mutter_x11_interop`.

## Installation

Download a binary for x86_64 Linux from
[Releases](https://github.com/hikuohiku/wl-ime-shim/releases).

With Nix:

```sh
nix run github:hikuohiku/wl-ime-shim -- --scale 2 discord
```

With Cargo (Rust 1.89 or later):

```sh
cargo install --locked --git https://github.com/hikuohiku/wl-ime-shim
```

## License

[MIT](LICENSE)
