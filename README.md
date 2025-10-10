# Ryokan

[![Build and test](https://github.com/walker84837/ryokan/actions/workflows/build.yml/badge.svg)](https://github.com/walker84837/ryokan/actions/workflows/build.yml)

> Ryokan, where your stray thoughts stay

Ryokan is a secure and private note-taking application designed for the command line. It allows you to create, manage, and encrypt your notes, ensuring your thoughts and ideas remain confidential.

## What does the name mean?

Ryokan (旅館) is named after traditional Japanese inns, a place where travelers temporarily store their belongings. Like its namesake, this app provides secure, encrypted storage for your thoughts and notes, keeping them protected (with PIN access) yet always available when you need them.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
  - [Prerequisites](#prerequisites)
  - [Building from Source](#building-from-source)
- [Usage](#usage)
  - [Command-Line Options](#command-line-options)
  - [TUI Keybindings](#tui-keybindings)
- [Configuration](#configuration)
- [How it Works](#how-it-works)
  - [Encryption & Decryption](#encryption--decryption)
  - [PIN Management](#pin-management)
- [License](#license)
- [Contributing](#contributing)
  - [Roadmap](#roadmap)
- [Acknowledgments](#acknowledgments)

## Features

- **Secure encryption**: Uses AES-256 GCM for encrypting note content with a randomly generated salt and nonce.
  
- **PIN protection**: Protect your notes with a 6-digit PIN for simplicity. The encryption key is derived from your PIN using [Argon2](https://en.wikipedia.org/wiki/Argon2).

- **TUI**: Navigate and preview notes in a nicely-built and cross-platform TUI.

- **External editor integration**: Edit your notes using your system’s default editor (or your preferred editor specified via command-line).

- **Configurable**: Customize the notes directory and configuration file location via command-line arguments.

- **Open source**: Distributed under either the [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) licenses.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)

### Building from Source

Clone the repository and build the project with Cargo:

```sh
git clone https://github.com/walker84837/ryokan.git
cd ryokan
cargo build --release
```

The executable will be located in `target/release/ryokan`.

## Usage

Launch Ryokan from your terminal:

```sh
./ryokan [OPTIONS]
```

### Command-line options

- `-n, --notes-dir <notes_dir>`: Specify the directory where your notes are stored. Defaults to `notes`.

- `-c, --config-file <config_file>`: Specify a custom configuration file path.

- `-e, --editor <editor>`: Specify the text editor to use. Defaults to the `EDITOR` environment variable, or falls back to `nano` if not set.

- `-v, --verbose`: Increase logging verbosity. You can use this flag multiple times for more detailed output.

### TUI keybindings

Once Ryokan is running, use the following keys to interact with the application:

- **Up/Down arrow keys**: Navigate through the list of notes.

- **Enter**: Open and edit the selected note. The note is decrypted to a temporary file, opened in your editor, and re-encrypted upon saving.

- **n**: Create a new note. A new, empty note file is generated with a unique UUID as its filename.

- **q**: Quit the application.

## Configuration

Ryokan stores its configuration (including the encrypted PIN hash) in a TOML file. By default, the configuration file is located in your operating system’s configuration directory:

- **Linux:** `~/.config/ryokan/ryokan.toml`
- **macOS:** `~/Library/Application Support/ryokan/ryokan.toml`
- **Windows:** `%APPDATA%\ryokan\ryokan.toml`

If no PIN is found when Ryokan starts, you will be prompted to set a new 6-digit PIN.

## How it works

### Encryption & decryption

- **Encryption:** When saving a note, Ryokan:
  
  1. Generates a 16-byte salt and a 12-byte nonce.
  2. Derives an encryption key from your 6-digit PIN using Argon2.
  3. Encrypts the note content using AES-256 GCM.
  4. Concatenates the salt, nonce, and ciphertext to form the encrypted note file.

- **Decryption:**  
  To open a note, Ryokan:
  1. Reads the salt and nonce from the encrypted file.
  2. Derives the key from your PIN using the extracted salt.
  3. Decrypts the ciphertext back into the original note content.

### PIN management

- **Setting a PIN**: If no PIN is stored in the configuration, you will be prompted to enter a 6-digit PIN. This PIN is then hashed (using Argon2) and stored in the configuration file.

- **Verifying a PIN**: When opening an existing note, your entered PIN is verified against the stored hash. If it doesn’t match, you will be prompted to try again.

## License

Ryokan is dual-licensed, and you may choose to use it under either of the following licenses:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

## Contributing

Contributions are welcome! If you have ideas for improvements or want to report a bug, please open an issue or submit a pull request on [GitHub](https://github.com/walker84837/ryokan).

### Roadmap

- [ ] Make note creation easier by ID'ing and encrypting notes which don't follow the same pattern as the others.
  - [ ] Not sure but, store in a key-value database the encrypted note ID and the original filename.

## Acknowledgments

Ryokan leverages several fantastic open-source libraries (you should check them out and give them a star):

- [clap](https://crates.io/crates/clap) for command-line argument parsing.
- [ratatui](https://crates.io/crates/ratatui) and [crossterm](https://crates.io/crates/crossterm) for building the terminal UI.
- [aes_gcm](https://crates.io/crates/aes_gcm) for AES-256 GCM encryption.
- [argon2](https://crates.io/crates/argon2) for secure PIN hashing and key derivation.
- [uuid](https://crates.io/crates/uuid) for generating unique filenames.
