# `noise_gui`

A graphical user interface for [Noise-rs](https://github.com/Razaekel/noise-rs).

TODO:

- [ ] Add node implementations for each `NoiseFn`
- [ ] Allow saving the graph project to a file
- [ ] Allow zoom/pan on the noise preview images
- [ ] Allow image/data export
- [ ] Warning when noise nodes won't render (not enough control points, etc: see expr.rs)
- [ ] Support wasm with online version
- [ ] Colors that differ input/output nodes and wires by type
- [ ] Automatic NoiseFn cached values
- [ ] Fix bugs

## Bugs

- Cyclic graphs: need to check for this and make sure users cannot do it!
- Connecting wrong things together

## Data model export

TBD: Once a graph has been completed you may export it to a file (RON?) and later reload the noise
graph and set named constants before evaluating points.
