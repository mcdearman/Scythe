//! The same grammar, the same bytes and the same method as `bench/`, so that
//! the two sets of numbers can be put next to each other.
//!
//! Best of a few runs rather than a mean: the work is deterministic, so the
//! spread is the machine's doing and the fastest run is the one it interfered
//! with least. `std::hint::black_box` keeps the optimiser from noticing that
//! nobody looks at the tokens.

use logos::Logos;
use std::hint::black_box;
use std::time::Instant;

/// The grammar as `bench/src/Lib.mw` declares it, variant for variant.
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip(r"--[^\n]*", allow_greedy = true))]
enum Token {
    #[token("+")] Plus,
    #[token("-")] Minus,
    #[token("*")] Star,
    #[token("/")] Slash,
    #[token("=")] Equals,
    #[token("(")] LParen,
    #[token(")")] RParen,
    #[token("let")] Let,
    #[regex(r"[0-9]+(\.[0-9]+)?")] Number,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")] Ident,
}

/// The token kinds alone: what logos does when no callback asks for the text.
fn lex_kinds(input: &str) -> usize {
    let mut lex = Token::lexer(input);
    let mut n = 0;
    while let Some(t) = lex.next() {
        black_box(&t);
        if t.is_ok() {
            n += 1;
        }
    }
    n
}

/// The same, taking the text of every token that carries one in the Meadow
/// version. `slice` borrows, so this allocates nothing -- which is the point of
/// reporting it separately.
fn lex_with_slices(input: &str) -> usize {
    let mut lex = Token::lexer(input);
    let mut n = 0;
    while let Some(t) = lex.next() {
        match &t {
            Ok(Token::Number) | Ok(Token::Ident) => {
                black_box(lex.slice());
            }
            _ => {}
        }
        black_box(&t);
        if t.is_ok() {
            n += 1;
        }
    }
    n
}

/// The same again, but building an owned `String` per token that carries text,
/// which is what the Meadow lexer has to do to put the text in the token.
fn lex_owned(input: &str) -> usize {
    let mut lex = Token::lexer(input);
    let mut n = 0;
    while let Some(t) = lex.next() {
        match &t {
            Ok(Token::Number) | Ok(Token::Ident) => {
                black_box(lex.slice().to_owned());
            }
            _ => {}
        }
        black_box(&t);
        if t.is_ok() {
            n += 1;
        }
    }
    n
}

fn best_of(reps: u32, mut act: impl FnMut() -> usize) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..reps {
        let t0 = Instant::now();
        black_box(act());
        let took = t0.elapsed().as_nanos();
        best = best.min(took);
    }
    best
}

fn report(name: &str, bytes: usize, nanos: u128) {
    let per_second = (bytes as u128) * 1_000_000_000 / nanos.max(1);
    println!(
        "{:<22}{:<12}{:<14}",
        name,
        format!("{} us", nanos / 1000),
        format!("{} KB/s", per_second / 1000)
    );
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "../corpus.calc".to_string());
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("could not read {path}: {e}");
            eprintln!("run this from `bench/rust`, or pass the path to corpus.calc");
            std::process::exit(1);
        }
    };
    let bytes = text.len();
    let tokens = lex_kinds(&text);
    println!("corpus.calc: {bytes} bytes, {tokens} tokens");
    println!();
    println!("{:<22}{:<12}{:<14}", "lexer", "best", "throughput");
    report("logos", bytes, best_of(200, || lex_kinds(&text)));
    report("logos, with slices", bytes, best_of(200, || lex_with_slices(&text)));
    report("logos, owned text", bytes, best_of(200, || lex_owned(&text)));
}
