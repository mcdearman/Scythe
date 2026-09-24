# scythe

A lexer written for you, from the tokens you declare.

```meadow
use Scythe (lexer!, token)

@derive(Lexer)
@skip("[ \t\r\n]+")
data Token
  = @token("+") Plus
  | @token("let") Let
  | @regex("[0-9]+") Number String
  | @regex("[a-zA-Z_][a-zA-Z0-9_]*") Ident String
```

That writes `lexToken : String -> Result Error [Lexed Token]` beside the
declaration:

```meadow
lexToken "let x 1 + 23"
-- Ok [Let, Ident("x"), Number("1"), Plus, Number("23")]
```

A port of the Rust crate [logos](https://github.com/maciejhirsz/logos), in the
shape Meadow's macros make possible.

## What it does

`@derive(Lexer)` reads the patterns off the variants and builds a deterministic
finite automaton **while your package is compiled** — the patterns are parsed,
turned into a nondeterministic machine, made deterministic by the subset
construction, then pruned and minimised, all inside the compiler. What is left
for run time is a table walk: no regex engine, no backtracking, one pass over
the bytes.

Two rules decide what a token is, and they are logos's:

* the **longest** match wins, so `letter` is an identifier rather than `let`
  followed by `ter`;
* where two patterns match the same text, the one **declared first** wins,
  which is what lets a keyword beat an identifier.

## Writing the declaration

| written | means |
|---|---|
| `@token("+")` | that text, exactly; its meta-characters are not special |
| `@regex("[0-9]+")` | a pattern |
| `@skip("[ \t]+")` | above the declaration: matched and thrown away |
| `Ident String` | a variant with a `String` field is given the text that matched |

The patterns are bytes, not characters: `[a-z]` is a range of bytes, and `.` is
any byte but a newline. What is supported is what a token looks like —
literals, classes (`[a-z0-9_]`, `[^ ]`), `\d` `\w` `\s` and their negations,
`*` `+` `?`, alternation, grouping. There are no captures, no backreferences
and no anchors: a lexer matches from where it stands and takes the longest
match, so there would be nothing for them to mean.

A pattern that is not a pattern is reported where you wrote it, at compile
time, and so is a variant that says nothing about what it matches.

## What you get

```meadow
@pub fun lexToken : String -> Result Error [Lexed Token]
```

* `token l`, `start l`, `stop l` — the token and the bytes it covers.
* `errorAt e` — where a scan stopped, when nothing matched.

## Using the machinery directly

The pieces are exported, so a lexer can be built without the derive:
`parse` reads a pattern, `compile` turns a numbered list of them into a `Dfa`,
and `scan` walks an input with one.

## What it does to the machine before it writes it down

The subset construction answers a machine that is correct, not one that is
small or quick. logos puts four passes between that machine and the code it
emits, and all four are here — in `src/Automaton.mw`, which says at the top
which is which:

* **dead ends are pruned.** A state that cannot reach an accepting state can
  only ever end in a backtrack, so the graph is walked backwards from the
  accepting states and everything it does not reach is thrown away. A scan that
  would have read to the end of a word before giving up now stops at the byte
  that made it hopeless.
* **equivalent states are merged, to a fixed point.** Two states that accept the
  same token and move the same way are one state. Merging a pair can leave two
  more identical, so the pass repeats until the count stops falling.
* **the alphabet is cut to byte classes.** Two bytes no pattern tells apart cost
  one column between them rather than two — logos's `ByteClass`, seen from the
  table's side. Columns that every state treats alike are merged as well.
* **dispatch is a lookup, not a search.** logos emits `TABLE[byte as usize]` for
  any state with more than two edges. The same idea laid out flat here:
  `trans[state * classes + classOf[byte]]`, two indexed reads and no comparison
  chain.

Two more are the scanner's rather than the automaton's. The input is taken
apart once, into an array — logos scans a `&[u8]`, and a persistent vector
would put a tree walk under every byte. And a token's text is cut only when
something wants it, which is logos's `Lexer::slice`: a skipped token —
whitespace, a comment — now costs nothing but the bytes it steps over.

There is one place this parts company with logos. logos writes a full 256-entry
table per state, which costs a Rust `const` nothing. Meadow gives every element
of an array literal its own register while the block that builds it runs, and
the allocator does not spill, so a literal of a few hundred numbers is refused
outright. A derive here writes its table as *text* and reads it back once, on
the first look at the `def` that holds it — a `def` is evaluated once and
remembered, so nothing per byte of input ever touches it.

## How fast

`bench/run.sh` lexes the same 48 KB of calculator source with this library and
with the Rust crate, the same way both times — best of a few runs, since the
work is deterministic and the spread is the machine's doing. Both agree the
file holds 10,270 tokens, which is what makes the columns worth comparing.
Meadow is built `--release` (`-O2`, compiled ahead of time) against Rust's
`--release`. On one laptop:

| lexer | best | throughput |
|---|---|---|
| meadow, before the passes above | 326 ms | 0.1 MB/s |
| meadow, after | 50 ms | 1.0 MB/s |
| meadow, after, token kinds only | 50 ms | 1.0 MB/s |
| rust logos | 91 µs | 523 MB/s |
| rust logos, borrowing each token's text | 98 µs | 487 MB/s |
| rust logos, building an owned `String` per token | 214 µs | 224 MB/s |

So the passes are worth about **6.5×**, and what is left is **236×** slower
than logos doing the same work — the last row is the like-for-like one, because
a Meadow token that carries its text carries a `String` it owns.

The remaining distance is the runtime, not the algorithm: both walk one table,
one byte at a time, and Meadow charges around 50 ns for an array read where a
compiled Rust program charges a load. The passes above are what stopped the
lexer paying that charge more often than it had to — a persistent vector read,
which the engine before them used for every byte of input, costs 2.8 µs.

## Example

`example/` is a calculator's lexer, with comments and floats. `meadow run` in
that directory.

`bench/` is the same grammar written four times — twice in Meadow, once for the
engine as it was, and once in Rust for logos — over one corpus. `./bench/run.sh`
runs the lot and prints the table above.

## Licence

MIT or Apache-2.0, at your option — the crate's own terms. See
[LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE) and
[COPYRIGHT](COPYRIGHT).
