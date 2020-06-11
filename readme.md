# Introduction

This is a Rust WASM app for Tomasulo GUI, not the course homework for the Computer Architecture course. 

Code structure:

- `src/inst.rs`: Define the nel instructions, as well as the parsing and formatting of them.
- `src/lib.rs`: Define the algorithm and data structures in the Tomasulo algorithm.
- `src/ser.rs`: Serialize the current state to a JSON string, and pass it to the JavaScript side.
- `www/index.html`: Define the elements on the web page. 
- `www/index.js`: Interact with the Rust side, using the JSON string from which to render those HTML elements.
  - I'm not familiar with frontend techniques at all, so if you have any complaints about my code, please tell me and maybe help me improve it.

# Build

```
# please first install rust compiler & wasm-pack & npm
# you can refer to https://rustwasm.github.io/docs/wasm-pack/quickstart.html
$ wasm-pack build
$ cd www
$ npm install
$ npm run start # now you can access localhost:8080 to use the simulator
```
