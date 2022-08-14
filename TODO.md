1. Fetch system once the “system” feature of “iced” becomes stable and included in main branch.

2. Add cross-fade support by providing fade-in/out duration (global, block, and local -- like how gain is set):
- use crossfade feature of rodio
- find similar feature on gstreamer

3. Add trigger support for video

4. Deploy progress bar at loading screen to give a sense of how long until block starts. Probably should be a progress of number of resources loaded (send message LoadProgress(done, total) on every successful load).

5. Solution to styling is to define CustomStyle structs with fields and a wrapper struct that has Option<T>  for each widget. Then deserialize from file with same name, and if Some(_) apply that style instead of the default one (in case of None).

6. Add text styling option. For now just font, color, and size.

7. Sequence is an action that either evolves into a list of actions that run in sequence, or has an internal quasi scheduler to handle messages. SequenceTemplate will be an action that only has a src, which will evolve into Sequence. Not allowed to define it within same file, to avoid cross-contamination. Build separate graphs for each sequence. Keeps each graph simple. Maybe just reuse scheduler without separate logger. Everything else should be the same?

8. Need to figure out routing. This will be useful for current use too. Maybe have a Vec<usize> or (de)queue/stack as “route”, that will give each action its relative address. This will be attached to all scheduler messages, so it goes to the right place. It can also be a string with : as delimiter: “13:10:74”, which will make use of join and split_once, but needs string conversion every time.

9. Stack can have an orientation and list of visual actions that may or may not be static but have to be visual, with a possible list of proportions along the main direction. It will also have an optional list of ending conditions which will determine what actions are sufficient to stop the container. It should forward update and view down to its children. Stack can evolve into StatefulColumn or StatefulRow for easier management.

10. Macro is an action that evolves into a set of actions. Flow should be only internal to macro action. Or maybe just have separate graph for this guy? Messages sent through to here, to be handled by internal logic.  2 Nops will tie together the start and end of the block. Might need to implement evolve for the dependency graph to allow growing number of nodes. Should track offset and shift all references to indices by offset. Check when name-to-id conversion is done.

11. Remove ExtAction and write a procedural macro that will implement name, id, log_when, etc. and their getters (part of some trait) which will be required by action. This simplifies the internal data structure, but I need to implement deserialization for Box<dyn Action>.

12. Make the logger a trait so users can implement their own versions. Maybe add a derive macro that takes care of the basics, which is optional.

13. Implement a logger with an embedded database, like SQLite or sled.
