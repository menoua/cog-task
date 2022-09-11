## To-do

- [x] Replace SVG icons with font icons.
- [x] Add trigger support for video (only if it has less than 2 audio channels).
- [ ] Implement ffmpeg backend for streaming.
- [ ] Build one of the media backends (probably `ffmpeg`) as a static dependency.
- [ ] Replace errors with `eyre` errors.
- [ ] Find alternative icon font to "font awesome" with open source thin/light icons. 
- [ ] Support audio fade-in/out by providing duration (global, block, and local -- like volume):
    - Use crossfade feature of rodio.
    - Find similar feature on gstreamer.
- [ ] Support spatial audio.
- [ ] Deploy progress bar at loading screen to give a sense of how long until block starts. Probably should be ratio of number of resources loaded (send message LoadProgress(done, total) on every successful load).
- [ ] Solution to styling is to define CustomStyle structs with fields and a wrapper struct that has Option<T>  for each widget. Then deserialize from file with same name, and if Some(_) apply that style instead of the default one (in case of None).
- [ ] Add text styling option. For now just font, color, and size.
- [ ] Add Complex/Macro action that is essentially a subgraph with its own scheduler.
- [ ] Need to figure out routing. This will be useful for current use too. Maybe have a Vec<usize> or (de)queue/stack as “route”, that will give each action its relative address. This will be attached to all scheduler messages, so it goes to the right place. It can also be a string with : as delimiter: “13:10:74”, which will make use of join and split_once, but needs string conversion every time.
- [ ] Stack can have an orientation and list of visual actions that may or may not be static but have to be visual, with a possible list of proportions along the main direction. It will also have an optional list of ending conditions which will determine what actions are sufficient to stop the container. It should forward update and view down to its children. Stack can evolve into StatefulColumn or StatefulRow for easier management.
- [ ] Remove ExtAction and write a procedural macro that will implement name, id, log_when, etc. and their getters (part of some trait) which will be required by action. This simplifies the internal data structure, but I need to implement deserialization for Box<dyn Action>.
- [ ] Make the logger a trait so users can implement their own versions. Maybe add a derive macro that takes care of the basics, which is optional.
- [ ] Implement a logger with an embedded database, like SQLite or sled.

## Waiting on upstream

- [ ] The problem of sometimes skipping screen updates seems to be connected to `winit` dropping certain messages. This seems to happen only when the update finishes too fast. As a workaround, I have added an up to 2ms delay to updates necessitating a refresh. ([#792](https://github.com/iced-rs/iced/issues/792), [#436](https://github.com/iced-rs/iced/issues/436))   
- [ ] The `iced/system` feature which allows fetching system information is merged into master and will be included in the next release of `iced`.
- [ ] The next release of `iced` will replace all widgets with their `pure` counterparts. Refactoring will be necessary. 
- [ ] When `iced_glow` supports image and SVG, implement a `glow`-based version for older systems. It seems to be generally more stable anyway.
- [ ] Text shaping support will hopefully come to `iced` in the future, which will allow better text support.
