# Crystal Runtime
A rust program utilizing webgpu to provide game engine like functionality.

Documentation to generate such games will be provided soon.

## Build instructions
### Native 
Step 1:

``cargo build --release``

Step 2:

``./target/release/crystal-runtime <.crg file>``
### WebAssembly
Step 1:

``cargo build --target wasm32-unknown-unknown --release``

Step 2:

``wasm-bindgen target/wasm32-unknown-unknown/release/crystal_runtime.wasm --target web --out-dir pkg``