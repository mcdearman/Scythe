# logos

A lexer written for you, from the tokens you declare.

```meadow
use logos (lexer!, token)

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
turned into a nondeterministic machine, and made deterministic by the subset
construction, all inside the compiler. What is left for run time is a table
walk: no regex engine, no backtracking, one pass over the bytes.

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

## Example

`example/` is a calculator's lexer, with comments and floats. `meadow run` in
that directory.

## Licence

MIT or Apache-2.0, at your option — the crate's own terms. See
[LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE) and
[COPYRIGHT](COPYRIGHT).
