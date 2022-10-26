## To-do

- [ ] Rename `sig_*` fields to `in_*` or `out_*` depending on direction.
- [ ] Change signal notation to `$[0x01]`.
- [ ] Unify all SignalId into a single state-updating signal. External communication will be handled by dedicated thread.
- [ ] Add button press count metric (`sig_count`) to `Reaction`.
- [ ] Each time a button is pressed that is captured by `Reaction`, emit. Only log once.
- [ ] Add `Counter` action which starts from an initial value and counts each time it receives a signal.
- [ ] Add `Math` action which takes in variables and logs/signals at start and every time the result changes. Candidates are: [meval](https://github.com/rekka/meval-rs), [savage](https://github.com/p-e-w/savage), [fasteval](https://github.com/likebike/fasteval), [caldyn](https://github.com/Luthaf/caldyn).
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
