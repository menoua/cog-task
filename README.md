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

## Description

This tool provides an easy way to write and execute different types of interactive actions that are usually useful in experiments involving cognitive sciences. E.g., display an image/video, play sounds, show text, measure reactions to events (through key presses or clicks), measure action completion times, ask questions, etc.

This application is written in [Rust](https://www.rust-lang.org/) using the [egui](https://github.com/emilk/egui) graphical framework. To generate a task, a description file in the rust object notation (RON) format should be created by the experiment designer. The task file consists of three main fields: name, configuration, and blocks (self-contained pieces of the experiment that should be run in one sitting). Each block in itself consists of three main fields: name, configuration (overriding the task configuration), and actions. The actions are specified in the form of a tree (graph) with nodes of type `Action`.

`Action`s are the fundamental building blocks of experiment design. There are many [types](https://github.com/menoua/cog-task/tree/master/src/action/core) of actions:
* Some actions are containers, i.e., they contain other actions within. Container actions are how the tree is constructed. For example, the action `Seq` is a sequence container which stores a list of sub-actions that will be run in sequence, one after the other. Another example is the `Par` action which is a parallel container, storing a list of sub-actions that will start at the same time (but might end at different times).
* Some actions are infinite which will never end on their own or through user interaction. These actions should be linked to other non-infinite actions. For example, `Timeout` is a container action that will run its inner sub-action for a fixed amount of time.
* Some actions do not have any effect on the experiment, but store the results. For example, `KeyLogger` stores key presses by the user and their times. Another example, `Logger` stores any information it receives from other actions into a file.
* ...

There are many more types of actions, which are not properly documented yet. But feel free to explore the [types](https://github.com/menoua/cog-task/tree/master/src/action/core) (each file corresponds to an action with the same name), or check out (and run) the multiple [examples](https://github.com/menoua/cog-task/tree/master/example).


## Installation

The most reliable way to install CogTask is by installing Cargo through [rustup](https://rustup.rs/) and compiling the binaries locally (check requirements section below).

Install Cargo:<br>
```bash
curl https://sh.rustup.rs -sSf | sh
```

Build binaries (choose one):
- Stable binaries from [crates.io](https://crates.io/crates/cog-task):<br>
  ```bash
  cargo install cog-task@1.0.0-beta [--features=...]
  ```
- Nightly binaries from [github](https://github.com/menoua/cog-task) (*preferred*):<br>
  ```bash
  cargo install --git https://github.com/menoua/cog-task [--features=...]
  ```

To update the installation to the latest version, you can run the same commands.

## Features

Some types of actions depend on optional features that can be enabled during installation. These features are not enabled by default because they rely on extra system libraries that might not be installed on the OS out-of-the-box.

Currently, there are 4 distinct features that can be enabled:
1. **audio** -- enables the `Audio` action via the ALSA sound library.
2. **gstreamer** -- enables the `Stream` and `Video` actions via the gstreamer backend (also enables **audio**).
3. **ffmpeg** (_incomplete_) -- enables the `Stream` and `Video` actions via the ffmpeg backend (also enables **audio**).
4. **savage** (_enabled by default_) -- enables using the [savage](https://github.com/p-e-w/savage) interpreter for `Math`.

Examples:
- Stable binaries with all features:<br>
  ```bash
  cargo install cog-task@1.0.0-beta --all-features
  ```
- Nightly binaries with **audio** and **gstreamer** support:<br>
  ```bash
  cargo install --git https://github.com/menoua/cog-task --features=audio,gstreamer
  ```

## Requirements

### macOS

|   Feature                   | Requirements |
| -----------                 | ------------ |
| (*required*)                | - |
| **audio**                   | - |
| **savage**                  | - |
| **gstreamer**               | `brew install gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav gst-rtsp-server` |
| **ffmpeg**                  | `brew install ffmpeg` |
| (*--all-features*)          | `brew install gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav gst-rtsp-server ffmpeg` |

### Linux

| Feature                     | Requirements |
| -------                     | ------------ |
| (*required*)                | `sudo apt install build-essential cmake pkg-config libfontconfig1-dev` |
| **audio**                   | `sudo apt install libasound2-dev` |
| **savage**                  | - |
| **gstreamer**               | `sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools gstreamer1.0-alsa gstreamer1.0-pulseaudio` |
| **ffmpeg**                  | `sudo apt install libavfilter-dev libavdevice-dev ffmpeg` |
| (*--all-features*)          | `sudo apt install build-essential cmake pkg-config libfontconfig1-dev libasound2-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools gstreamer1.0-alsa gstreamer1.0-pulseaudio libavfilter-dev libavdevice-dev ffmpeg` |

## Usage

This crate installs two binaries: `cog-launcher` and `cog-server`.

`cog-launcher` is a launcher that provides a graphical interface to find and load tasks from disk.

`cog-server /path/to/task` is used to run a specific task by providing the path to its directory. `cog-launcher` runs this binary when starting a task, so make sure both binaries are in the same directory.

For example, to run the [**Basic**](https://github.com/menoua/cog-task/tree/master/example/basic/) task in this repo, you would do the following:
```bash
git clone https://github.com/menoua/cog-task
cog-server cog-task/example/basic
```

Alternatively, you can run:
```bash
cog-launcher
```
Then use the leftmost control icon to load the [*example/basic/*](https://github.com/menoua/cog-task/tree/master/example/basic/) directory. Or, you can use the second button to open the parent [*example/*](https://github.com/menoua/cog-task/tree/master/example/) directory which contains all the example tasks within. The former, directly runs `cog-server` on the chosen task. The latter, displays a list of all tasks located in the chosen directory, which can be started by clicking the corresponding button.
