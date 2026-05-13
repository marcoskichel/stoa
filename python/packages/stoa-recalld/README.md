# stoa-recalld

Long-lived Python daemon that hosts MemPalace in-process and serves
Stoa's Rust hooks + CLI over a Unix domain socket.

Wire protocol: newline-delimited JSON. One request per connection.
Methods: `search`, `mine`, `write_wiki`, `read_wiki`, `health`.

See `stoa daemon start` to launch and `stoa daemon status` to probe.
