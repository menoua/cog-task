## To-do

- [ ] Save the binaries generated for macOS and Linux by CI for specific cases (base, audio, audio+gstreamer, full).
- [ ] Add a `Process` function which runs an externally compiled binary and maintains a comms channel. Use [this](https://play.rust-lang.org/?code=%23!%5Ballow(unused)%5D%0Afn%20main()%20%7B%0Ause%20std%3A%3Aio%3A%3AWrite%3B%0Ause%20std%3A%3Aprocess%3A%3A%7BCommand%2C%20Stdio%7D%3B%0A%0Alet%20mut%20child%20%3D%20Command%3A%3Anew(%22rev%22)%0A%20%20%20%20.stdin(Stdio%3A%3Apiped())%0A%20%20%20%20.stdout(Stdio%3A%3Apiped())%0A%20%20%20%20.spawn()%0A%20%20%20%20.expect(%22Failed%20to%20spawn%20child%20process%22)%3B%0A%0Alet%20mut%20stdin%20%3D%20child.stdin.take().expect(%22Failed%20to%20open%20stdin%22)%3B%0Astd%3A%3Athread%3A%3Aspawn(move%20%7C%7C%20%7B%0A%20%20%20%20stdin.write_all(%22Hello%2C%20world!%22.as_bytes()).expect(%22Failed%20to%20write%20to%20stdin%22)%3B%0A%7D)%3B%0A%0Alet%20output%20%3D%20child.wait_with_output().expect(%22Failed%20to%20read%20stdout%22)%3B%0Aassert_eq!(String%3A%3Afrom_utf8_lossy(%26output.stdout)%2C%20%22!dlrow%20%2ColleH%22)%3B%0A%7D&edition=2021) code base.
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
