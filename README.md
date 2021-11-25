# Language Alchemist
An engine for procedurally generating conlangs and translations for conlangs. Written in Rust with [`egui`](https://github.com/emilk/egui).

## Work in Progress
This project is a work in progress. Many features are not fully implemented.

## Usage
Requires Rust 2021 edition or later. Build and run with:
```
$ cargo run
```

## Concept
You can create multiple conlangs, each with a set of adjustable parameters pertaining to their lexical, orthographic, morphological, and syntactic features. Once you customize a language's features, you can ask the engine to provide translations for arbitrary text. The engine "fills in" unknown words by generating translations on the fly, according to the features you adjusted. Once a word is generated, it's saved to a lexicon so that the engine never produces different translations for the same input.

There are two modes of use: **basic mode** and **expert mode**.

* In basic mode, you enter raw text and the engine provides a simple translation. Because natural language processing is still a developing field of research, the generated translations are usually very simple and their syntax closely resembles that of the input text.

* In expert mode, you must add linguistic annotations to the input text before translation. In exchange, the engine can generate significantly "smarter" translations with programmable grammar rules. The annotations take three forms:

  * **Type annotations** indicate the part of speech (or other category) of a word. This can be inferred for most function words, determiners, pronouns, and words that have an unambiguous attribute annotation. Example: `I see#v a dog#n`.
  
  * **Attribute annotations** specify additional metadata about a word, including tense, plurality, and more. This helps the engine see relationships between words. For instance, instead of writing `ate`, you could write `eat.PST` to indicate that "ate" isn't a new word, but just the past tense of "eat". These annotations are optional, but make the translations more realistic. Example: `I will find the answers` becomes `I find.FUT the answer.PL`.

  * **Group annotations** denote a simplified form of the input's syntax tree. Simply wrap each noun phrase that spans more than one word (determiners can be ignored) in parentheses. Example: `I want#v (a bright#a red#a biycle#n)`.