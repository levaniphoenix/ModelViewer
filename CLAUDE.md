# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build          # compile
cargo run            # run the viewer (loads scenes/lumine.glb at startup)
cargo check          # fast type-check without linking
cargo clippy         # lint
```

There are no tests yet.

## Architecture

This is a Rust desktop 3D model viewer built on **eframe 0.34** (egui) with a **wgpu** render backend.

### Startup flow

`main.rs` → loads a glTF file into `Vec<MeshPrimitive>` → creates `eframe::run_native` → inside the `CreationContext` callback, `App::new` uploads all primitives to the GPU via `Resources::new` and stores them in `egui_wgpu::CallbackResources`.

### Rendering model

eframe owns the wgpu `Device`, `Queue`, and the main render pass. 3D rendering is injected into egui's render pass via `egui_wgpu::Callback::new_paint_callback` (see `app.rs:render_3d_viewport`). This means:

- **`ViewportCallback::prepare`** — updates the camera uniform buffer on the GPU (called before the render pass opens).
- **`ViewportCallback::paint`** — sets the pipeline, bind group, vertex/index buffers and draws all primitives inside egui's already-open render pass.

There is no separate render pass or offscreen texture — the 3D scene is drawn directly into egui's swapchain render pass as a callback.

### Key types

| Type | File | Role |
|---|---|---|
| `Resources` | `renderer.rs` | GPU-side state: pipeline, uniform buffer, bind group, all `GpuPrimitive`s |
| `GpuPrimitive` | `renderer.rs` | Vertex buffer + index buffer for one glTF primitive |
| `Camera` / `CameraUniform` | `camera.rs` | Orbit camera; builds view-projection matrix sent to the shader |
| `Vertex` / `MeshPrimitive` | `mesh.rs` | CPU-side mesh data; `load_gltf` reads positions + normals from a glTF file |
| `App` | `app.rs` | egui app; owns yaw/pitch/radius orbit state, handles drag/scroll input |

### Shader (`shaders/scene.wgsl`)

Single vertex+fragment shader. Vertex inputs are `@location(0) position: vec3<f32>` and `@location(1) normal: vec3<f32>`, matching `Vertex::buffer_layout()`. The fragment stage does a simple diffuse + ambient shading with a hardcoded directional light.

### Depth testing

The pipeline requires `wgpu::TextureFormat::Depth32Float`. eframe must be configured with `depth_format: Some(...)` in `WgpuConfiguration` (in `main.rs`) so the render pass egui creates includes a depth attachment.
