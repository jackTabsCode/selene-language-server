# selene-language-server

This is a very simple language server for the Lua & Luau linter, [Selene](https://github.com/Kampfkarren/selene)!

It supports the following features:
`initialize`, `shutdown`, `textDocument/didOpen`, `didChange`, `didClose`, `publishDiagnostics`, `codeAction`

It should properly forward diagnostics from Selene to the client. I also implemented a code action to insert Selene "allow" comments.

## Installation

- Install the Rust compiler with `rustup`
- Clone this repository
- Install the language server with Cargo:
	- `cargo install --path crates/language-server`
	- You should now have a `selene-language-server` binary in your Cargo bin directory
- Configure it for your editor (see below)

### Zed

- Add the `wasm32-wasip2` toolchain:
	- `rustup target add wasm32-wasip2`
- From the extensions page in Zed, click the Install Zed Extension button, and select the `crates/zed` directory

The extension does not install Selene or the language server for you. Make sure Zed is able to see both binaries.

### VSCode

You should probably just use the [official extension](https://marketplace.visualstudio.com/items?itemName=Kampfkarren.selene-vscode) instead.

### Other Editors

I don't know, but it should be straightforward because it implements LSP.
