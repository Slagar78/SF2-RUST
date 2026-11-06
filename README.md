# Shining Force II: Native Rust Port (SF2-Rust)

**SF2-Rust** is a native source port of the classic game *Shining Force II* (Sega Genesis, 1993), written in Rust and built on SDL3.

The project is not a console emulator. Its goal is a complete reconstruction of the game engine based on the [SF2DISASM](https://github.com/ShiningForceCentral/SF2DISASM) disassembly. Game logic is translated from Motorola 68000 assembly into Rust, decoupling the port from the original Video Display Processor (VDP) and enabling a modern software renderer.

**Data-Driven Engine Recreation**

## Tech Stack

* **Language:** Rust (Edition 2021)
* **Graphics / Input:** SDL3
* **Data source:** Original `.bin` assets from SF2DISASM

## Prerequisites and Setup (MSYS2 UCRT64)

The project is developed in the MSYS2 UCRT64 environment.

1. **Install dependencies in MSYS2:**

   Open the MSYS2 UCRT64 terminal and run:

   ```bash
   pacman -S mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-pkg-config mingw-w64-ucrt-x86_64-SDL3 ```

## Подготовьте ассеты:

Скачайте репозиторий SF2DISASM (нажмите split.bat) и скопируйте папку disasm/data в ваш проект по пути src/data.

Запуск:
Из терминала MSYS2 UCRT64 в корне проекта:

bash
cargo run