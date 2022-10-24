## To-do

- [ ] Separate examples into "base", "audio", "gstreamer", "ffmpeg", and "full". So there would be something to run regardless of chosen features.
- [ ] Consider relegating compile-time asset management to [rust-embed](https://github.com/pyrossh/rust-embed).
- [ ] Improve error messages by taking advantage of `eyre`'s contextualized error reports.
- [ ] Build a proper documentation for developers and users alike.
- [ ] Currently infinite recursion in task definition is (automatically) handled by stack overflow, but a better error that doesn't crash the entire `Server` would be preferrable. This is specific to `Template`, maybe it can be handled internally by `Template`.
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

## Waiting on upstream

- [ ] Text shaping support will hopefully come to `egui` (and the Rust ecosystem) in the future, which will allow better text support. Though this could take a very long time.
