# Bot Arena

[![Bot Arena CI](https://github.com/sdeming/botarena/actions/workflows/botarena.yml/badge.svg)](https://github.com/sdeming/botarena/actions/workflows/botarena.yml)

![Bot Arena](https://raw.githubusercontent.com/sdeming/botarena/main/screenshot.png)

A programmable robot battle simulator written in Rust. Program your own battle bots in a custom stack-based assembly language (RASM), then watch them fight in a dynamic, obstacle-filled arena! Supports up to 4 robots per match, real-time rendering, and detailed logging for debugging and analysis.

This game is heavily inspired by [CRobots by Tom Poindexter](https://tpoindex.github.io/crobots/)

It was written with massive amounts of AI assistance using both RooCode and Cursor combined with various models as a fun little experiment that might have gotten a little out of hand.

There are likely many bugs and other oddities that only an AI could explain. Or attempt to explain while seretly deleting random files for gits and shiggles. To be honest, it was a very frustrating but fulfilling experience.

---

## Features

- **Custom Assembly Language (RASM):** Write stack-based programs to control your robot's movement, scanning, and combat.
- **Real-Time Simulation:** Visualize battles in a 20x20 grid arena with obstacles, projectiles, and particle effects.
- **Flexible Logging:** Adjustable log levels and topic-based debug filtering for deep inspection of robot behavior.
- **Deterministic VM:** Each robot runs in its own virtual machine, cycle-by-cycle, with strict instruction costs and resource limits.
- **Extensive Documentation:** Full language reference in [LANGUAGE.md](LANGUAGE.md), with example bots and programming tips.
- **Robust Testing:** 179+ unit and integration tests ensure correctness and stability.

---

## Quick Start

### Prerequisites

- Rust (edition 2024, recommended latest stable)

### Build & Run

```sh
# Clone the repo
git clone https://github.com/yourusername/botarena.git
cd botarena

# Build and run with two example robots
cargo run -- bots/chaos.rasm bots/jojo.rasm
```

#### Command-Line Options

```
Usage: botarena [OPTIONS] <ROBOT_FILES>...

Arguments:
  <ROBOT_FILES>...  Paths to the robot program files (up to 4)

Options:
  -m, --max-turns <MAX_TURNS>        Maximum number of turns for the simulation [default: 1000]
      --log-level <LOG_LEVEL>        Log level (off, error, warn, info, debug, trace) [default: info]
      --debug-filter <DEBUG_FILTER>  Optional comma-separated list of targets for debug/trace logging
      --no-obstacles                 Whether to place obstacles in the arena
      --no-audio                     Disable sound effects
  -h, --help                         Print help
  -V, --version                      Print version
```

Example:

```sh
cargo run -- bots/chaos.rasm bots/jojo.rasm --turns=500 --log-level=debug --debug-filter=vm,robot
```

---

## Logging & Debugging

- **Log Levels:** Set with `--log-level` (off, error, warn, info, debug, trace).
- **Debug Filters:** Use `--debug-filter` to restrict debug output to specific topics (e.g., `vm`, `robot`, `drive`, `weapon`, `scan`, `instructions`).
- **Log Output:** All logs are printed to stdout. To capture logs for analysis:

```sh
cargo run -- bots/chaos.rasm bots/jojo.rasm --log-level=debug > debug.log 2>&1
```

- **Log Format:**
  - Timestamps, log level, robot/turn/cycle context, topic, and message.
  - Example: `[12:34:56.789] DEBUG [R01][T005][C10] vm: Executed instruction: push 1.0`

---

## Writing Robots (RASM)

Robots are programmed in **RASM**, a stack-based assembly language designed for Bot Arena. Each robot runs in its own VM, controlling movement, scanning, and combat via instructions.

- **Key Features:**

  - 19 general-purpose registers (`@d0`–`@d18`), status and component registers
  - 1024 memory cells, stack operations, subroutines, and control flow
  - Direct control of drive and turret components (move, rotate, scan, fire)
  - Strict instruction cycle costs (e.g., `fire` = 3 cycles, `drive` = 2 cycles)
  - Constants, labels, and expressions for program organization

- **Constraints:**

  - Max 10-level call stack
  - 100 cycles per turn, 1000 turns max (configurable)
  - Arena: 1.0 x 1.0 units (20x20 grid, 800x800 pixels)
  - Up to 4 robots per match
  - See [LANGUAGE.md](LANGUAGE.md) for full details

- **Getting Started:**
  - See [LANGUAGE.md](LANGUAGE.md) for a complete guide, including example programs, instruction set, and best practices.
  - Example bot files: `bots/chaos.rasm`, `bots/jojo.rasm`

---

## Arena & Game Constraints

- **Arena Size:** 1.0 x 1.0 units (20x20 grid, 800x800 pixels)
- **Obstacles:** Randomly placed (1% density by default) -- currently turned off
- **Turns:** 1000 max (default, configurable)
- **Cycles per Turn:** 100
- **Robot Health:** 100.0 (default)
- **Robot Power:** 1.0 (regenerates at 0.01 per cycle)
- **Drive/Turret Rotation:** 90° per turn
- **Projectile Speed:** 0.2 units/cycle
- **Scanner FOV:** 22.5° (±11.25°), range covers arena diagonal
- **See [src/config.rs](src/config.rs) for all tunable parameters**

---

## License

MIT License. See [LICENSE](LICENSE) for details.

---

## Documentation & Resources

- [LANGUAGE.md](LANGUAGE.md): Full RASM language reference and programming guide
- [src/config.rs](src/config.rs): Arena/game configuration
- Example bots: `bots/`

---

## Attributions

Some resources used in botarena were created by others. This could be anything from graphics, fonts, audio files, etc.

### Fonts
- [Retrauhaus](https://www.fontspace.com/retrahaus-font-f23785) by 538 Fonts

### Sound effects and audio files
- [Kenny](https://www.kenney.nl) from the all-in-one package

---

If you happen to stumble across this and decide to take a stab at writing a bot or two (it's tough, but fun!), please
consider opening a PR so I can include your bot.

Or, if you decide decide to take on fixing or improving the code, please do so! There are a lot of things _wrong?_ with
this code so I would love to see it improved.

Happy bot battling!
