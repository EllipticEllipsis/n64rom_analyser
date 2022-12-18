# n64rom_analyser
Find code and attempt to determine the compiler, microcode, libultra version, compression, etc.

- [src/findcode](src/findcode/) is mostly a Rust reimplementation of [findcode](https://github.com/decompals/findcode/).
- [src/compiler.rs](src/compiler.rs) is a collection of heuristics for determining which compiler(s) a game might have used (although currently is only actually good at distinguishing GCC and IDO)
- [src/compression.rs](src/compression.rs) covers various compression algorithms that are easy to spot (currently Yaz0, Yay0, MIO0)
- [src/ngrams.rs](src/ngrams.rs) crude first attempt at an ngrams library
