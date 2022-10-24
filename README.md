# CogTask

[![Crates.io Version](https://img.shields.io/crates/v/cog-task.svg)](https://crates.io/crates/cog-task)
[![Crates.io Downloads](https://img.shields.io/crates/d/cog-task.svg)](https://crates.io/crates/cog-task)
[![Build Status](https://github.com/menoua/cog-task-rs/workflows/CI/badge.svg)](https://github.com/menoua/cog-task-rs/actions)
[![License](https://img.shields.io/crates/l/cog-task.svg)](https://opensource.org/licenses/MIT)

A general-purpose low-latency application to serve cognitive tasks, built with [egui](https://github.com/emilk/egui).

## Installation

The most reliable way to install CogTask is by installing Cargo through [rustup](https://rustup.rs/) and compiling the binaries locally.

Install Cargo:<br>
```
$ curl https://sh.rustup.rs -sSf | sh
```

Build binaries (choose one):
- Stable binaries from [crates.io](https://crates.io/crates/cog-task):<br>
  ```
  $ cargo install cog-task [--features=...]
  ```
- Nightly binaries from [github](https://github.com/menoua/cog-task-rs):<br>
  ```
  $ cargo install --git https://github.com/menoua/cog-task-rs [--features=...]
  ```

## Features

By default (no features), this package should compile and run out-of-the-box on a reasonably recent macOS or Linux distribution. Some types of actions however depend on features that can be enabled during installation. These features are not enabled by default because they rely on system libraries that might not be installed on the OS out-of-the-box.

Currently, there are 4 main features that can be enabled:
1. **audio** -- enables the `Audio` action via the ALSA sound library.
2. **gstreamer** -- enables the `Stream` and `Video` actions via the gstreamer backend.
3. **ffmpeg** -- enables the `Stream` and `Video` actions via the ffmpeg backend.
4. **full** -- a shorthand to enable the previous three features.

Examples:
- Stable binaries with full support:<br>
  ```
  $ cargo install cog-task --features=full
  ```
- Nightly binaries with **audio** and **gstreamer** support:<br>
  ```
  $ cargo install --git https://github.com/menoua/cog-task-rs --features=audio,gstreamer
  ```

## Requirements:

Some features depend on certain libraries that might not come preinstalled on your OS. In these cases, before building the binaries with that features enabled, you need to first install the requirements:

### audio

On *linux*, requires installing ALSA, e.g.:<br>
```bash
$ sudo apt install libasound2-dev pkg-config
```

### gstreamer

On *macOS*, requires installing gstreamer, e.g.:<br>
```bash
$ brew install gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav gst-rtsp-server
```

On *linux*, requires installing gstreamer, e.g.:<br>
```bash
$ sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools gstreamer1.0-alsa gstreamer1.0-pulseaudio libavfilter-dev libavdevice-dev
```

## Usage

This crate installs two binaries -- `cog-launcher` and `cog-server`.

`cog-launcher`: A launcher that provides a graphical interface to find and load tasks from disk.

`cog-server /path/to/task`: Used to run a specific task by providing the path to its directory. `cog-launcher` runs this binary when starting a task, so make sure both binaries are in the same directory.

For example, to run the [***Basic***](https://github.com/menoua/cog-task-rs/tree/master/example/basic) task in this repo, you would do the following:
```bash
$ git clone https://github.com/menoua/cog-task-rs
$ cog-server cog-task-rs/example/dummy
```

Alternatively, you can run:
```bash
$ cog-launcher
```
Then use the leftmost control icon to load a specific task directory. Or, you can use the second button to load a parent directory which contains task directories within. The first option, directly runs `cog-server` on the chosen task. The second option, displays a list of all tasks located in the chosen directory, which can be started by clicking the corresponding button.

## Changelog

Version 0.2.0 has gone through a massive overhaul, transitioning from the GUI framework of `iced` to `egui`. The transition was done to solve a screen update skipping issue (which it did). There have been other pros and cons too:

- Text and widget styling is (much) more difficult in `egui`.
- `egui`'s Glow backend supports image/SVG.
- Separating the `view` and `update` calls allowed redesigning block architecture (the dependency graph) into an action tree. This change makes it very difficult to design a buggy task, and significantly simplifies task definition style. It slightly limits the task design flexibility, but it's worth it. This change also comes with an increased overhead since `update`/`view` calls traverse the entire active subset of the tree, instead of jumping to the end nodes. However, the tree overhead is generally low compared to action-specific overheads, so that's not a huge deal.
