# Marlin PC Simulator Setup Guide

Marlin firmware supports a native simulation environment (`BOARD_SIMULATED`) which compiles the firmware code into a standard desktop executable rather than flashing it onto physical microcontroller hardware. This enables developers to dry-run serial print commands, inspect display UI, and test G-code execution.

---

## 1. Prerequisites

1. Install **Visual Studio Code (VS Code)**.
2. Install the **PlatformIO IDE** extension within VS Code.
3. Download the [Marlin Firmware Source Code](https://github.com/MarlinFirmware/Marlin).

---

## 2. Configuration Setup

Within the Marlin source directory, configure the PlatformIO environment for simulator target compilation:

1. **Modify `platformio.ini`**:
   Find the default build environment setting and change it to the simulated target for your development OS:
   ```ini
   default_envs = simulator_windows
   # (or simulator_linux, simulator_macos depending on your host OS)
   ```

2. **Configure Motherboard in `Marlin/Configuration.h`**:
   Define the motherboard hardware profile as `BOARD_SIMULATED` to instruct Marlin to compile software mocks for the heaters, steppers, and thermal sensors:
   ```cpp
   #define MOTHERBOARD BOARD_SIMULATED
   ```

3. **Configure LCD Controller (Optional)**:
   By default, the simulator emulates a graphical smart controller (like the REPRAP_DISCOUNT_FULL_GRAPHIC_SMART_CONTROLLER) rendered in an OpenGL desktop window. You can verify it is active in `Configuration.h`:
   ```cpp
   #define REPRAP_DISCOUNT_FULL_GRAPHIC_SMART_CONTROLLER
   ```

---

## 3. Building and Running

1. Open the Marlin source folder in VS Code.
2. Click the **PlatformIO** icon in the sidebar.
3. Select **Build** (under your selected simulator env).
4. Select **Upload** or **Run** to execute the binary.
   - An OpenGL window will launch displaying the simulated printer LCD screen.
   - A command line window will open, acting as the console serial interface of the simulated printer.

---

## 4. Connecting printproof3d-adapters

To test the Marlin adapter against the running simulator:
- **Windows**: Use a virtual COM port pair utility (e.g., com0com) to link one virtual COM port to the simulator and the other COM port to your `printproof3d` serial configuration.
- **Linux/macOS**: Use `socat` to create a virtual serial port pair:
  ```bash
  socat -d -d pty,raw,echo=0 pty,raw,echo=0
  ```
  Point the simulator's target PTY config to the first terminal endpoint, and set your `printproof3d` serial port to the second.
