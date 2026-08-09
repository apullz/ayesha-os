---
name: code-review
description: Use when the user asks to review code, find bugs, or check the quality of engine sources
---

# code review

Follow these steps in order when reviewing Rust code in the engine:

1. Locate the code under review. Use glob `**/*.rs` to find files if the target is unknown.
2. Read the file(s) with read_file. Read the whole file, not fragments.
3. Check for, in priority order:
   - unsoundness: unwrap() on user input, indexing that can panic, unchecked paths
   - resource leaks: handles, ports, threads that never stop
   - async issues: blocking calls inside async fns, missing awaits, unbounded loops
   - error handling: swallowed errors, `let _ =` on results that matter
   - Windows-specific traps: path separators, case sensitivity, readonly attributes
4. Grep for suspicious patterns before claiming the code is clean:
   - grep pattern "unwrap()" path "engine/src"
   - grep pattern "TODO|FIXME|HACK" path "engine/src"
5. Report findings as a list: severity (critical/warning/nit), file:line, what's wrong, suggested fix.
6. Never rewrite code unless the user asks. Always propose, then wait.
