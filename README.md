# CogTask

A general-purpose low-latency application to serve cognitive tasks, built with [egui](https://github.com/emilk/egui).

## Installation

The most reliable way to install CogTask is by installing Cargo through [rustup](https://rustup.rs/) and compiling the binaries locally.

Install cargo: ```curl https://sh.rustup.rs -sSf | sh```

Build stable binaries from [crates.io](https://crates.io/crates/cog-task): ```cargo install cog-task```

**OR** Build unstable binaries from [github](https://github.com/menoua/cog-task-rs): ```cargo install --git https://github.com/menoua/cog-task-rs```

## Usage

This crate installs two binaries -- `cog-launcher` and `cog-server`.

`cog-launcher`: A launcher that provides a graphical interface to find and load tasks from disk.

`cog-server /path/to/task`: Used to run a specific task by providing the path to its directory. `cog-launcher` runs this binary when starting a task, so make sure both binaries are in the same directory.

## Changelog

Version 0.2.0 has gone through a massive overhaul, transitioning from the GUI framework of `iced` to `egui`. The transition was done to solve a screen update skipping issue (which it did). There have been other pros and cons too. Text and widget styling is (much) more difficult in `egui`. `egui`'s Glow backend supports image/SVG. Separating the `view` and `update` calls allowed redesigning block architecture (the dependency graph) into an action tree. This change makes it very difficult to design a buggy task, and significantly simplifies task definition style. It slightly limits the task design flexibility, but it's worth it. This change also comes with an increased overhead since `update`/`view` calls traverse the entire active subset of the tree, instead of jumping to the end nodes. However, the tree overhead is generally low compared to action-specific overheads, so that's not a huge deal.
