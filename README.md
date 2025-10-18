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

Install the Clang toolchain with the WASI libc++ runtime (for example `apt-get install clang libc++-20-dev-wasm32 libc++abi-20-dev-wasm32`) and expose them to the build scripts:

```
export CC=clang
export CXX=clang++
export AR=llvm-ar-20
export RANLIB=llvm-ranlib-20
export NM=llvm-nm-20
export CXXFLAGS="--sysroot=/usr/lib/llvm-20/lib/wasm32-wasi -D__wasi__ -D__wasm32__ -isystem/usr/lib/llvm-20/include/wasm32-wasi/c++/v1 -isystem/usr/include/wasm32-wasi/c++/v1 -isystem/usr/include/wasm32-wasi -fexceptions"
export LIBRARY_PATH=/usr/lib/llvm-20/lib/wasm32-wasi:/usr/lib/wasm32-wasi
export CXXSTDLIB_wasm32_unknown_unknown=c++
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C target-feature=+exception-handling -C link-arg=-L/usr/lib/llvm-20/lib/wasm32-wasi -C link-arg=-L/usr/lib/wasm32-wasi -C link-arg=-lc++ -C link-arg=-lc++abi"
```

``cargo build --target wasm32-unknown-unknown --release``

Step 2:

``wasm-bindgen target/wasm32-unknown-unknown/release/crystal_runtime.wasm --target web --out-dir pkg``