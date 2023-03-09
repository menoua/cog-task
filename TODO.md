## To-do

- [ ] The `rodio` backend does not support playback of audio with more than two channels through macOS's "Aggregate Device" which combines different output devices into a single virtual device with more channels.
- [ ] Add Pointer action which wraps its internal element and records the position of the click. If an optional mask image is provided, it will measure if click was within mask. Sends out reaction_time, accuracy, and position.
- [ ] Introduce global variables at the `Server` level which populate the state of scheduler at start, and are written back to Server only after successful completion.
- [ ] Introduce "requires" attribute to `Block` which means a block can only run after the required blocks. This shouldn't be used for order since it can't be circumvented if necessary.
- [ ] Save the binaries generated for macOS and Linux by CI for specific cases (base, audio, audio+gstreamer, full).
- [ ] Improve error messages by taking advantage of `eyre`'s contextualized error reports.
- [ ] Build a proper documentation for developers and users alike.
- [ ] Consider replacing the current message broadcast system with a spmc channel (check out the "bus" crate).
- [ ] Consider relegating compile-time asset management to [rust-embed](https://github.com/pyrossh/rust-embed).
- [ ] Fix `ffmpeg` implementation. Currently, this backend is missing a lot of functionality (sound, looping, trigger, etc.).
- [ ] Build one of the media backends (probably `ffmpeg`) as a static dependency.
- [ ] Find alternative icon font to "font awesome" with open source thin/light icons. 
- [ ] Support audio fade-in/out by providing duration (global, block, and local -- like volume):
    - Use crossfade feature of rodio.
    - Find similar feature on gstreamer.
- [ ] Add styling option for certain actions/widgets.
- [ ] Make the logger a trait so users can implement their own versions. Maybe add a derive macro that takes care of the basics, which is optional.
- [ ] Implement a logger with an embedded database, like SQLite or sled.
- [ ] Improve the default widgets styles.
- [ ] Add option for audio cross-fade just in case.
- [ ] A persistent (across `Server` instantiation) channel to external programs will be needed to handle communication with recording devices, etc.
- [ ] Add `Direction` task: show virtual head with Left/Right/Front marked on screen. Two modes: Continuous and Quantized(n: u32). If continuous, do math and draw line wherever mouse is pointing. If quantized, divide space into n equal sized slices. Pointer selects slice. Allow for limiting the range of angles? At least front-only (180°) and front-and-back (360°).

## Waiting on upstream

- [ ] Text shaping support will hopefully come to `egui` (and the Rust ecosystem) in the future, which will allow better text support. Though this could take a very long time.
