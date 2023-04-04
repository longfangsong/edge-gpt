# edge-gpt

A Rust version of [EdgeGPT](https://github.com/acheong08/EdgeGPT).

In addition to the original features, we:

- enable the user to store the session into a JSON and restore it, which enables continuing an existing chat when deploying as a serverless function.

- export the `source_attribution` and `suggested_responses` provided by Bing.

See [this example](./examples/continually/main.rs) for how to use it.
