# knowledge-engine-rs
A chatbot that can learn and reason about what you tell it.

## Running
```
cargo run --release
```

## Current supported phrases:
```
All <a> are <b>.
Some <a> are <b>.
<a> are <b>.
<a> are not <b>.
Tell me about <a>.
```

## Example:
```
Hello!
Tell me something about your world!
> Trees are green.
Hmmm...
Ok!
> Trees are tall.
Hmmm...
Ok!
> Trees are plants.
Hmmm...
Ok!
> Plants are alive.
Hmmm...
Ok!
> Trees are not green.
Hmmm...
That doesn't seem right...
> Tell me about trees.
Hmmm...
All trees are alive.
All trees are green.
All trees are plants.
All trees are tall.
```

## Roadmap:
- Intelligent rule eviction
  - When the solver resolves as UNSAT, (a logical conflict, currently resulting in "That doesn't seem right..."), search for the minimum set of rules that if removed could make it once again satisfiable.
- More phrases
  - `Is <a> <b>?`
  - `<a> that are <b> are [not] c.`
- Resolving pronouns
```
> I am cool.
Hmm...
Ok!
> Tell me about myself.
Hmm...
You are cool.
```
