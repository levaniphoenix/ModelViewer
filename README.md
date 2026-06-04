# Model Viewer

A desktop 3D model viewer built with Rust, [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) (egui), and [wgpu](https://wgpu.rs/). Loads and renders glTF (`.glb`) files with orbit camera controls and simple diffuse lighting.

## Usage

```bash
cargo run
```

By default, the viewer loads `scenes/lumine.glb` at startup. Drag to orbit, scroll to zoom.

## Requirements

- Rust toolchain (stable)
- A GPU with Vulkan, DX12, or Metal support
