# A lil Rust resource-based task scheduler, for use in game engines 

Just a simple **opinionated** toy library to try implementing a custom system that resembles shred but with resources being described using runtime bitmasks instead of typed generics. Inspired to implement this after seeing how unoptimized and single threaded my game engine was, so hopefully this could be a drop-in replacement to my current task scheduler.

Now I know that this is mostly used for "tasks" but I'm going to call them "systems" because that's what I used in cFlake and in the source code here. Don't think this has to do anything with operating systems though.

## Features
* Supports closures and just plain functions as systems.
* Resource (group) based scheduler. Avoids conflicts by sorting systems according to their "depth" and resource read/write bits.
* Global world where you can access resources without lock contentation (since the scheduler prevents it).
* Supports up to an arbitrary number of thread, but allows you to limit them (and force some systems that *could* run in parallel to run sequentially)
* Injection rules that allow some systems to run before others
