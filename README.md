<div align="center">

<img src="LOGO.svg" height="128px"  alt="rusty brain logo"/>

# CogTask

[![Crates.io](https://img.shields.io/crates/v/cog-task.svg)](https://crates.io/crates/cog-task)
[![Documentation](https://docs.rs/cog-task/badge.svg)](https://docs.rs/cog-task)
[![License](https://img.shields.io/crates/l/cog-task.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/menoua/cog-task/workflows/CI/badge.svg)](https://github.com/menoua/cog-task/actions)
[![Crates.io Downloads](https://img.shields.io/crates/d/cog-task.svg)](https://crates.io/crates/cog-task)

A general-purpose low-latency tool for designing cognitive tasks.

</div>

## Intro

This tool provides an easy way to write and execute different types of interactive actions that are usually useful in experiments involving cognitive sciences. E.g., display an image/video, play sounds, show text, measure reactions to events (through key presses or clicks), measure action completion times, ask questions, etc.

This application is written in [Rust](https://www.rust-lang.org/) using the [egui](https://github.com/emilk/egui) graphical framework. To generate a task, a description file in the rust object notation (RON) format should be created by the experiment designer. The task file consists of three main fields: name, configuration, and blocks (self-contained pieces of the experiment that should be run in one sitting). Each block in itself consists of three main fields: name, configuration (overriding the task configuration), and actions. The actions are specified in the form of a tree (graph) with nodes of type `Action`.

`Action`s are the fundamental building blocks of experiment design. There are many [types](https://github.com/menoua/cog-task/tree/master/src/action/core) of actions:
* Some actions are containers, i.e., they contain other actions within. Container actions are how the tree is constructed. For example, the action `Seq` is a sequence container which stores a list of sub-actions that will be run in sequence, one after the other. Another example is the `Par` action which is a parallel container, storing a list of sub-actions that will start at the same time (but might end at different times).
* Some actions are infinite which will never end on their own or through user interaction. These actions should be linked to other non-infinite actions. For example, `Timeout` is a container action that will run its inner sub-action for a fixed amount of time.
* Some actions do not have any effect on the experiment, but store the results. For example, `KeyLogger` stores key presses by the user and their times. Another example, `Logger` stores any information it receives from other actions into a file.
* ...

There are many more types of actions, which are not properly documented yet. But feel free to explore the [types](https://github.com/menoua/cog-task/tree/master/src/action/core) (each file corresponds to an action with the same name), or check out (and run) the multiple [examples](https://github.com/menoua/cog-task/tree/master/example).


## Installation

The most reliable way to install CogTask is by installing Cargo through [rustup](https://rustup.rs/) and compiling the binaries locally.

Install Cargo:<br>
```bash
$ curl https://sh.rustup.rs -sSf | sh
```

Build binaries (choose one):
- Stable binaries from [crates.io](https://crates.io/crates/cog-task):<br>
  ```bash
  $ cargo install cog-task@1.0.0-beta [--features=...]
  ```
- Nightly binaries from [github](https://github.com/menoua/cog-task):<br>
  ```bash
  $ cargo install --git https://github.com/menoua/cog-task [--features=...]
  ```

## Update

To update the installation to the latest version, add the `-U` option to whichever of the installation commands you used:
  ```bash
  $ cargo install -U [...]
  ```

## Features

By default (no features), this package should compile and run out-of-the-box on a reasonably recent macOS or Linux distribution. Some types of actions however depend on features that can be enabled during installation. These features are not enabled by default because they rely on system libraries that might not be installed on the OS out-of-the-box.

Currently, there are 4 main features that can be enabled:
1. **audio** -- enables the `Audio` action via the ALSA sound library.
2. **gstreamer** -- enables the `Stream` and `Video` actions via the gstreamer backend.
3. **ffmpeg** (_incomplete_) -- enables the `Stream` and `Video` actions via the ffmpeg backend.
4. **full** -- a shorthand to enable the previous three features.

Examples:
- Stable binaries with full support:<br>
  ```bash
  $ cargo install cog-task@1.0.0-beta --features=full
  ```
- Nightly binaries with **audio** and **gstreamer** support:<br>
  ```bash
  $ cargo install --git https://github.com/menoua/cog-task --features=audio,gstreamer
  ```

## Requirements:

Some features depend on certain libraries that might not come preinstalled on your OS. In these cases, before building the binaries with said features enabled, you need to first install the requirements:

### audio

On *linux*, requires installing pkg-config and ALSA, e.g.:<br>
```bash
$ sudo apt install pkg-config libasound2-dev
```

### gstreamer

On *macOS*, requires installing gstreamer, e.g.:<br>
```bash
$ brew install gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav gst-rtsp-server
```

On *linux*, requires installing libgstreamer, e.g.:<br>
```bash
$ sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools gstreamer1.0-alsa gstreamer1.0-pulseaudio libavfilter-dev libavdevice-dev
```

### ffmpeg

On *linux*, requires installing pkg-config and libavutil, e.g.:<br>
```bash
$ sudo apt install pkg-config libavfilter-dev libavdevice-dev
```
  - *NOTE: Although it is not a requirement at compile time, to be able to use the ffmpeg backend, you need to have the ffmpeg library installed on the system during runtime.*

## Usage

This crate installs two binaries -- `cog-launcher` and `cog-server`.

`cog-launcher`: A launcher that provides a graphical interface to find and load tasks from disk.

`cog-server /path/to/task`: Used to run a specific task by providing the path to its directory. `cog-launcher` runs this binary when starting a task, so make sure both binaries are in the same directory.

For example, to run the [**Basic**](https://github.com/menoua/cog-task/tree/master/example/basic/) task in this repo, you would do the following:
```bash
$ git clone https://github.com/menoua/cog-task
$ cog-server cog-task/example/basic
```

Alternatively, you can run:
```bash
$ cog-launcher
```
Then use the leftmost control icon to load the [*example/basic/*](https://github.com/menoua/cog-task/tree/master/example/basic/) directory. Or, you can use the second button to open the parent [*example/*](https://github.com/menoua/cog-task/tree/master/example/) directory which contains all the example tasks within. The former, directly runs `cog-server` on the chosen task. The latter, displays a list of all tasks located in the chosen directory, which can be started by clicking the corresponding button.
