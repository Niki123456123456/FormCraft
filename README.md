# FormCraft

A browser-based 3D workspace built in Rust with **Bevy 0.20** and **egui**. The first milestone imports and inspects GLB models; mesh creation and editing are future work.

## Run locally

Install Rust through [rustup](https://rustup.rs/) and [Trunk](https://trunkrs.dev/):

```sh
cargo install --locked trunk --version 0.21.14
trunk serve
```

Open <http://127.0.0.1:8080>. The checked-in toolchain file installs Rust 1.97.1 and its WASM target. First compilation takes several minutes. A current desktop browser with WebGL 2 enabled is required. No backend or account is needed.

You can also run `cargo run` for a native window. Linux builds need X11 development libraries (`libx11-dev libxkbcommon-dev`).

## Controls

| Action | Control |
| --- | --- |
| Orbit | Left or right mouse drag |
| Pan | Shift + left drag, or middle mouse drag |
| Zoom | Mouse wheel or trackpad scroll |
| Change projection | Perspective / Orthographic buttons |
| Standard views | Click a face, beveled edge, or beveled corner of the orientation cube |
| Reset view | Home |
| Fit models | Frame all, or select a hierarchy node and Frame selection |
| Toggle construction grid | Grid |
| Import | Import GLB; multiple files can be selected |
| Remove a model | Select any of its hierarchy nodes, then Remove |

The grid uses Y up, with red X, green Y, and blue Z axes. A grid square is one scene unit. Imported geometry is centered over XZ and grounded on Y=0. Units, materials, textures, transforms, and node names within the model are preserved. Importing another file adds a model; it does not replace existing imports. Files with the same name remain distinct.

The hierarchy shows the default glTF scene, or the first scene when no default is specified, with expandable descendants. Unnamed nodes receive a fallback label. Selecting a node lets you frame that subtree; Remove removes its entire imported model.

## GLB support

After selecting files in the picker, click **OK** to import them.

Files are read into an in-memory Bevy asset source and loaded with Bevy's standard glTF loader, including embedded PNG/JPEG textures and PBR materials. Files never leave the device. Reloading clears the workspace.

Use self-contained `.glb` files with embedded buffers and textures. External sidecar files and Draco/meshopt compressed geometry are not supported. Invalid files and asset-loading failures appear in the UI. Only the default/first scene is instantiated. Imported cameras and lights are disabled so the workspace remains under the editor camera and lighting. Animations are loaded but not played automatically.

## GitHub Pages

1. Create a GitHub repository and push this directory to its `main` branch.
2. In **Settings → Pages → Build and deployment**, select **GitHub Actions** as the source.
3. The **Build and deploy FormCraft** workflow checks formatting, runs GLB validation tests, compiles the release WASM through Trunk, and deploys the static `dist/` artifact.

Pull requests build and test without deploying. You can also trigger a deployment from the Actions tab. Relative asset URLs support both `https://<owner>.github.io/<repository>/` and custom domains. For a different deployment branch, change the branch in `.github/workflows/pages.yml`.

Build locally for static hosting:

```sh
trunk build --release --locked --public-url ./
python3 -m http.server 8080 --directory dist
```

## Dependencies and choices

- [Bevy 0.20](https://bevy.org/news/bevy-0-20/): rendering, glTF, ECS, and gizmos. Its built-in `PanOrbitCamera` originated in the CAD-focused `bevy_editor_cam`, so a separate orbit plugin is unnecessary.
- [bevy_egui](https://github.com/vladbat00/bevy_egui): egui integration for the toolbar, hierarchy, dialogs, and view cube.
- [Bevy Assets](https://bevy.org/assets/): camera alternatives reviewed include [bevy_panorbit_camera](https://github.com/Plonq/bevy_panorbit_camera) and [bevy_editor_cam](https://github.com/aevyrie/bevy_editor_cam). The integrated controller avoids an additional compatibility dependency.
- Bevy gizmos draw the construction grid. The small orientation cube is drawn by egui from the inverse camera rotation; no third-party art is required.
- [rfd](https://github.com/PolyMeilex/rfd): browser and native file picker.
- [Trunk](https://trunkrs.dev/): WASM bindings, asset hashing, optimization, and local development server.

```sh
cargo fmt --all -- --check
cargo check --locked --target wasm32-unknown-unknown
cargo test --locked --no-default-features --lib
```

A nested GLB fixture is included at `tests/fixtures/assembly.glb` (regenerate with `python3 scripts/generate_fixture.py`). For the optional real-browser smoke test, install Playwright and run `node scripts/browser-smoke.cjs` after building. It checks startup, file picking, orbit, zoom, projection controls, and browser errors, and saves screenshots to `/tmp/formcraft-smoke`. Set `CHROME_PATH` to use an installed Chrome executable.
