# scythe

A lexer written for you, from the tokens you declare.

```meadow
use Scythe (lexer!, token)

@derive(Lexer)
data Token
  = @regex("[ \t\r\n]+") @skip Whitespace
  | @token("+") Plus
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

A variant marked `@skip` is matched like any other and left out of what is
lexed. Leave the `@skip` off and it is a token too — `Whitespace` between every
pair above — which is what a lossless syntax tree is built from. (A pattern
with no variant of its own can also be skipped from the type,
`@skip("--[^\n]*")`.)

A port of the Rust crate [logos](https://github.com/maciejhirsz/logos), in the
shape Meadow's macros make possible.

## What it does

`@derive(Lexer)` reads the patterns off the variants and builds a deterministic
finite automaton **while your package is compiled** — the patterns are parsed,
turned into a nondeterministic machine, made deterministic by the subset
construction, then pruned and minimised, all inside the compiler. Then it
writes the machine out as code, as logos does: a function per state, each
reading one byte and jumping to the next. No regex engine, no backtracking,
no table to consult, one pass over the bytes.

Two rules decide what a token is, and they are logos's:

- the **longest** match wins, so `letter` is an identifier rather than `let`
  followed by `ter`;
- where two patterns match the same text, the one **declared first** wins,
  which is what lets a keyword beat an identifier.

## What you get

```meadow
@pub fun lexToken : String -> Result Error [Lexed Token]
@pub fun foldToken : (acc -> Token -> Int -> Int -> acc) -> acc -> String -> Result Error acc
@pub fun nextToken : String -> Int -> Result Error (Maybe (Lexed Token))
```

- `lexToken` — every token, with where it was.
- `foldToken f acc input` — the tokens one at a time, into
  `f acc token start stop`, keeping none of them. This is logos's `Lexer`, an
  iterator, and it is the one to reach for when nothing needs every token at
  once: it skips building the vector, which is most of what `lexToken` costs.
- `nextToken input at` — the one token at byte `at`, past anything skipped,
  or `None` at the end. logos's `Lexer::next` after a `bump`: what a lexer is
  made of that reads part of a text with one set of tokens and part with
  another, as logos's `morph` does -- the text inside a string literal, say,
  and the code in its `${…}` holes.
- `token l`, `start l`, `stop l` — the token and the bytes it covers.
- `errorAt e` — where a scan stopped, when nothing matched.

## Using the machinery directly

The pieces are exported, so a lexer can be built without the derive:
`parse` reads a pattern, `compile` turns a numbered list of them into a `Dfa`,
and `scan` (or `foldScan`) walks an input with one. That scanner walks the
tables rather than code written for them, so it is the slower road.

## What it does to the machine before it writes it down

The subset construction answers a machine that is correct, not one that is
small or quick. logos puts passes between that machine and the code it emits,
and these are here, in `src/Automaton.mw`, which says at the top which is
which:

- **dead ends are pruned.** A state that cannot reach an accepting state can
  only ever end in a backtrack, so the graph is walked backwards from the
  accepting states and everything it does not reach is thrown away. A scan that
  would have read to the end of a word before giving up now stops at the byte
  that made it hopeless.
- **equivalent states are merged, to a fixed point.** Two states that accept the
  same token and move the same way are one state. Merging a pair can leave two
  more identical, so the pass repeats until the count stops falling.
- **the alphabet is cut to byte classes.** Two bytes no pattern tells apart cost
  one column between them rather than two — logos's `ByteClass`, seen from the
  table's side. Columns that every state treats alike are merged as well.

All of it runs inside the compiler, under the step budget Meadow gives a macro,
so it is written to do each piece of work once. Every state's closure over
empty edges, the letter each byte falls in, and what each state accepts are
worked out before the subset construction starts, not inside it. Each state of
the machine is then made in one pass over its edges, where the first version
asked every letter about every edge. The table is filled edge by edge rather
than cell by cell. A 16-pattern grammar with strings, comments, hex and floats
used to run out of budget, and now builds in less than half the time (24 ms to
11 ms, measured natively).

## How it writes the machine down

What `src/Gen.mw` writes is logos's shape:

- **a function per state.** Moving from one state to the next is a call in
  tail position, which compiles to a jump. The machine is the program's control
  flow, not data the program reads.
- **a jump table per state.** A state picks where to go by the byte's class,
  with a chain of tests on one `Int` that LLVM turns into a `switch`, and a
  dense `switch` into a jump table. Classes that lead to the same state share
  an arm.
- **lookup tables, packed.** A byte's class is one byte of a 256-byte string,
  read with `stringByteAt`, which is one load. A state with an edge back to
  itself — an identifier, a number, a run of spaces — gets one _bit_ of a
  second table, which says of each byte whether the state stays put on it.
  That is logos's fast loop, and its trick of packing several such tables into
  the bits of one: seven to a byte here, because a string is UTF-8 and a lone
  byte above 127 would not survive.
- **one word per match.** A state answers where the longest match ended and
  which token it was as one `Int`, so nothing is allocated until a token is.
- **the input is read where it is.** A Meadow string is bytes already, so it is
  never copied into an array; the text of a token is cut from it only for a
  token that is kept and carries its text, which is logos's `Lexer::slice`.

Everything written has its numbers annotated `Int`, and its tables annotated
too. A number nothing pins down is passed as any `Num`, with its arithmetic
looked up in a dictionary; and a state that never reads the class table would
otherwise be generic in it, and every call to it an instantiation.

## How fast

Expect, compiled with `--release --runtime aot`:

| what you call                     | throughput   |
| --------------------------------- | ------------ |
| `foldKind` — kinds, iterated      | **~60 MB/s** |
| `foldToken` — with their text     | ~44 MB/s     |
| `lexKind` — collected             | ~43 MB/s     |
| `lexToken` — collected, with text | ~35 MB/s     |

and on the default runtime (`meadow run --release`), about **25 MB/s**
iterating and 12 MB/s collecting.

Those are `bench/`'s numbers. `bench/run.sh` lexes the same 48 KB of
calculator source with this library and with the Rust crate, the same way both
times — best of several runs, since the work is deterministic and the spread is
the machine's doing. Both agree the file holds 10,270 tokens, which is what
makes the rows worth comparing. Meadow is built with `--release --runtime aot`
(LLVM, `-O2`) against Rust's `--release`. On one Windows laptop:

| lexer                                            | best    | throughput |
| ------------------------------------------------ | ------- | ---------- |
| meadow, the first engine (`before` in the bench) | 38.5 ms | 1.3 MB/s   |
| meadow, `lexToken`                               | 1.4 ms  | 35 MB/s    |
| meadow, `lexKind` (no token carries text)        | 1.1 ms  | 43 MB/s    |
| meadow, `foldToken`                              | 1.1 ms  | 44 MB/s    |
| meadow, `foldKind`                               | 810 µs  | 61 MB/s    |
| rust logos                                       | 76 µs   | 645 MB/s   |
| rust logos, borrowing each token's text          | 79 µs   | 618 MB/s   |
| rust logos, building an owned `String` per token | 175 µs  | 282 MB/s   |

`foldKind` against `logos` is the like-for-like pair — both iterate, neither
keeps a token — and it is about **10×** behind. With text, `foldToken` against
logos building an owned `String` is about **6×** behind, since a Meadow token
that carries its text carries a string of its own.

What is left is per token, not per byte: the state functions walk a byte in a
couple of loads and a jump, but each token costs three calls that are not in
tail position -- into the machine, into the constructor, into your function --
and a native Meadow call costs a few nanoseconds more than a Rust one.

How it got here. The table walk this replaced took 164 ms on the same file,
built by the same compiler as it was then: the code generation above took that
to 2.8 ms, and fixes to the compiler it exposed took it to 1.4 ms -- local
loops lifted to the top level, `match` on literals and on every constructor
compiled without failure objects, a branch's context given a join point rather
than a closure, and an array grown in place when nothing else holds it.

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
