<div align="center">

<img src="LOGO.png" height="100px"  alt="rusty brain logo"/>

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

This application is written in [Rust](https://www.rust-lang.org/) using the [egui](https://github.com/emilk/egui) graphical framework. To generate a task, a description file in the rust object notation ([RON](https://github.com/ron-rs/ron); see "Tooling" section of its README for syntax highlighting) format should be created by the experiment designer. The task file consists of three main fields: name, configuration, and blocks (self-contained pieces of the experiment that should be run in one sitting). Each block in itself consists of three main fields: name, configuration (overriding the task configuration), and actions. The actions are specified in the form of a tree (graph) with nodes of type `Action`.

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
  cargo install cog-task [--features=...]
  ```
- Nightly binaries from [github](https://github.com/menoua/cog-task) (*preferred*):<br>
  ```bash
  cargo install --git https://github.com/menoua/cog-task [--features=...]
  ```

To update the installation to the latest version, you can run the same commands.

## Features

Some types of actions depend on optional features that can be enabled during installation. These features are not enabled by default because they rely on extra system libraries that might not be installed on the OS out-of-the-box.

Currently, there are 5 distinct features that can be enabled:
1. **rodio** -- allows playing sounds via the CoreAudio sound library on macOS and ALSA on linux.
2. **gstreamer** -- allows streaming audio/video files via the gstreamer backend.
3. **ffmpeg** (_incomplete_) -- allows streaming audio/video files via the ffmpeg backend.
4. **savage** -- enables using the [savage](https://github.com/p-e-w/savage) interpreter for mathematical operations.
5. **python** -- enables using python code snippets to perform calculations.

Examples:
- Stable binaries with all features:<br>
  ```bash
  cargo install cog-task --all-features
  ```
- Nightly binaries with **rodio** and **gstreamer** support:<br>
  ```bash
  cargo install --git https://github.com/menoua/cog-task --features=rodio,gstreamer
  ```

## Requirements

### macOS

|   Feature                   | Requirements |
| -----------                 | ------------ |
| (*required*)                | - |
| **rodio**                   | - |
| **savage**                  | - |
| **gstreamer**               | `brew install gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav gst-rtsp-server` |
| **ffmpeg**                  | `brew install ffmpeg` |
| **python**                  | (needs a working python installation; see below) |
| (*--all-features*)          | `brew install gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav gst-rtsp-server ffmpeg` |

### Linux

| Feature                     | Requirements |
| -------                     | ------------ |
| (*required*)                | `sudo apt install build-essential cmake pkg-config libfontconfig1-dev` |
| **rodio**                   | `sudo apt install libasound2-dev` |
| **savage**                  | - |
| **gstreamer**               | `sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools gstreamer1.0-alsa gstreamer1.0-pulseaudio` |
| **ffmpeg**                  | `sudo apt install libavfilter-dev libavdevice-dev ffmpeg` |
| **python**                  | (needs a working python installation; see below) |
| (*--all-features*)          | `sudo apt install build-essential cmake pkg-config libfontconfig1-dev libasound2-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-tools gstreamer1.0-alsa gstreamer1.0-pulseaudio libavfilter-dev libavdevice-dev ffmpeg` |

### //@ python

Enabling the **python** feature can be tricky. You need a working installation of python3, which generally comes preinstalled with recent versions of both macOS and Linux. If you installed python using anaconda, you generally don't need to do anything else. If you installed python using a different method, you might need to set up the PYTHONHOME environment variable manually. The variable needs to be set to the location of the desired python environment:
```bash
export PYTHONHOME=path_to_python_env
```

This should be set before running `cog-launcher` or `cog-server` in the same shell environment. It might take some trial and error to get it going.

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

## Changelog

The SemVer version will follow these guidelines: If the new version is backwards compatible (task written for last version will behave the same on the new version), even if there are (1) new action types, or (2) new attributes for an existing action type introduced, the third number will increase. If an existing action type is removed entirely or an existing action's attributes (or their default values) have changed such that it is no longer backwards compatible, the second number will increase. If there is a fundamental change to the structure of the program (how tasks/actions are defined or executed), the first number will increase. Bug fixes will generally increase the third number, unless they are big, in which case they will increase the second number.

**v1.1.4**:
- `Process` now has a `drop_early` attribute which if set will drop/ignore all incoming responses before the corresponding action starts. This is incompatible with response_type of raw_all.
- Fixed a bug in `Porcess` which if multiple responses were received before action started, only one was consumed, keeping a list of unconsumed responses forever.

**v1.1.3**:
- Using a python function no longer requires manually setting the `PYTHONHOME` environment variable IF python has been set up using anaconda.

**v1.1.2**:
- New action `Stack` runs actions in parallel and displays them either in a horizontal or vertical stack.
- New action `Horizontal` is a shorthand for a horizontal `Stack`.
- New action `Vertical` is a shorthand for a vertical `Stack`.

**v1.1.1**:
- `Process` now has a `passive` attribute which if set does not send anything to child process.
- `Process` now has a `response_type` attribute which determines whether response content should be read: (a) value = read a line and convert it to claimed type, (b) raw = read a line and treat it as string, (c) raw_all = read full output and treat it as a single string.

**v1.1.0**:
- New action `Process` that can run an externally compiled binary in blocking or non-blocking mode. 
- `Function` now has a `lo_response` attribute which if set, will run in non-blocking mode.
- `Function` attribute `persistent` is replaced by the opposite attribute `once`, which if true will only run the function once (be it at start or update).
- Fixed a bug in `Delayed`.

**v1.0.1**:
- `Clock` now sends incrementing tic number instead of a null in its output signal.
- `Clock` has `on_start` attribute that determines whether a "zero" signal will be emitted at start of action.
