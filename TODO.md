## To-do

- [ ] Verify all signals are connected. Add a collect_signals(..) -> (in, out) function and verify they are equal as sets.
- [ ] Add a `Math`-like action that takes in the path to an external compiled FFI (C/Rust/Python/Matlab/etc.) and behaves very similarly to `Math`. This is for when you have functions written in a different language. This is only used for eval. For FfiPython if typed calling doesn't work can implement an interpreter calling with PyO3 and populate the function call arguments with string replacement like in Instruction. Candidates: [cpython](https://github.com/dgrunwald/rust-cpython) for calling user-defined python, [cc](https://dev.to/xphoniex/how-to-call-c-code-from-rust-56do) for including C functions at compile time.
- [ ] Add a `Process` function which runs an externally compiled Rust/C/Python and maintains a comms channel.
- [ ] Improve error messages by taking advantage of `eyre`'s contextualized error reports.
- [ ] Build a proper documentation for developers and users alike.
- [ ] Consider relegating compile-time asset management to [rust-embed](https://github.com/pyrossh/rust-embed).
- [ ] Fix `ffmpeg` implementation. Currently this backend is missing a lot of functionality (sound, looping, trigger, etc.).
- [ ] Build one of the media backends (probably `ffmpeg`) as a static dependency.
- [ ] Find alternative icon font to "font awesome" with open source thin/light icons. 
- [ ] Support audio fade-in/out by providing duration (global, block, and local -- like volume):
    - Use crossfade feature of rodio.
    - Find similar feature on gstreamer.
- [ ] Add styling option for certain actions/widgets.
- [ ] Make the logger a trait so users can implement their own versions. Maybe add a derive macro that takes care of the basics, which is optional.
- [ ] Implement a logger with an embedded database, like SQLite or sled.
- [ ] Improve the default widgets styles.
- [ ] Add an external function action (`Ffi`). `StatefulFfi` runs the proper application in a separate thread and maintains a communication link with it. Emitted `Signal`'s will be sent to the thread using that comm link. The `Ffi` will send back data through a similar communication link.
- [ ] Add option for audio cross-fade just in case.
- [ ] A persistent (across `Server` instantiation) channel to external programs will be needed to handle communication with recording devices, etc.
- [ ] Add `Direction` task: show virtual head with Left/Right/Front marked on screen. Two modes: Continuous and Quantized(n: u32). If continuous, do math and draw line wherever mouse is pointing. If quantized, divide space into n equal sized slices. Pointer selects slice. Allow for limiting the range of angles? At least front-only (180°) and front-and-back (360°).
- [ ] Docs.rs fails to build due to missing dependencies. There must be a way to solve this. For now, disabled all features on the docs.rs build.
- [ ] Connected with the docs build issue, `nix`+`flake` might be the solution to both building docs and also taking care of external dependencies (ALSA, gstreamer, ffmpeg).

## Waiting on upstream

- [ ] Text shaping support will hopefully come to `egui` (and the Rust ecosystem) in the future, which will allow better text support. Though this could take a very long time.
