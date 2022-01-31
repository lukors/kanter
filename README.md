# Kanter
A node based texture creation tool for Windows and Linux. It uses [Kanter Core](https://github.com/lukors/kanter_core) for the computation and [Bevy](https://github.com/bevyengine/bevy) for the graphical user interface.

[**Download the latest version here**](https://github.com/lukors/kanter/releases/latest)

![kanter_0-2-0](https://user-images.githubusercontent.com/1719884/117169645-908b1d80-adc9-11eb-9aee-6815c34d3f53.png)

## Design Goals
- [x] Responsive - Fast to start and fluid to use
- [ ] Simple - Easy to pick up, depth for power users
- [ ] Capable - Able to generate every kind of texture

## Current Goal
Our strategy is to use feedback, testing, and our design goals to guide our efforts towards the most valuable work.

**The current goal is to release [Alpha 3](https://github.com/lukors/kanter/milestone/4), which includes support for our first use case: manual image channel packing.**

## Features
It's fast, but not much else, it does not cover any use cases since it is too buggy and incomplete to use comfortably.

### Nodes
- Input: Loads an image from disk
- Output: Saves an image to disk when selected and `Shift` `Alt` `S` is pressed
- Separate: Takes an RGBA image and splits it into 4 grayscale images
- Combine: Takes 4 grayscale images and merges them into an RGBA image
- Value: Outputs a given value

### Other
- Instructional text in the program to guide the user
