# `noise_gui`

[![GitHub](https://img.shields.io/badge/github-attackgoat/noise__gui-blue?logo=github)](https://github.com/attackgoat/noise_gui)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/attackgoat/noise_gui/blob/master/LICENSE-MIT)
[![Apache](https://img.shields.io/badge/license-Apache-blue.svg)](https://github.com/attackgoat/noise_gui/blob/master/LICENSE-APACHE)
[![GitHub Pages](https://img.shields.io/github/actions/workflow/status/attackgoat/noise_gui/main.yml)](https://github.com/attackgoat/noise_gui/actions/workflows/main.yml)

---

A graphical user interface for [Noise-rs](https://github.com/Razaekel/noise-rs).

> [!TIP]
> `noise_gui` runs on Linux/Mac/Windows desktops and [**the web**](https://attackgoat.github.io/noise_gui/)!

![Demo](.github/img/demo.gif "Demo")

## Features:

- [x] Support for all [Noise-rs](https://github.com/Razaekel/noise-rs) `NoiseFn` implementations
- [ ] Allow zoom/pan on preview images
- [x] Allow saving the graph project to a file (_desktop only_)
- [ ] Allow image/data export
- [ ] Automatic `NoiseFn` cached values
- [x] WASM support using [Trunk](https://trunkrs.dev/)

> [!WARNING]
> `noise_gui` is currently in the proof-of-concept phase and may contain bugs and missing features.

## Development Dependencies

Ubuntu 22.04:

```bash
sudo apt install libgtk-3-dev
```

Browser:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## How To Run Locally

Desktop:

```bash
cargo run
```

Browser:

```bash
trunk serve --open
```

## Data model export

TBD: Once a graph has been completed you may export it to a file (RON?) and later reload the noise
graph and set named constants before evaluating points.
