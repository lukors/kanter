# Kanter
A node based texture creation tool for Windows and Linux. It uses [Kanter Core](https://github.com/lukors/kanter_core) for the computation and [Bevy](https://github.com/bevyengine/bevy) for the graphical user interface.

[**Download the latest version here**](https://github.com/lukors/kanter/releases/latest)

![kanter_0-2-0](https://user-images.githubusercontent.com/1719884/117169645-908b1d80-adc9-11eb-9aee-6815c34d3f53.png)

## Goals
- Responsiveness - It should be fast to start, and snappy to use
- Simplicity - It should be easy to understand and not be bloated with unnecessary nodes or cluttered UI
- Completeness - It should have the tools to generate every kind of texture

## Features
In its current pre-alpha state Kanter can be used for simple tasks like packing textures, inverting channels in an image, and changing the alpha channel of an image.

It is very clunky and has many rough corners, but it gets the job done.

### Nodes
- Input: Loads an image from disk
- Mix: Mathematically combines two inputs
- Value: Outputs a given value
- Output: Saves an image to disk when selected and `Shift Alt S` is pressed

### Other
- Basic node manipulation
- Instructional text in the program to guide the user

## Roadmap
The focus right now is on beefing up the backend library that this application uses, so the artist won't have as many technical limitations. Then the plan is to build out basic functionality like save/load graphs, a set of basic nodes and make it nice to use. That's as far as the roadmap goes right now.

I keep most planned tasks as issues on GitHub, so check those to see what's coming up.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.