# `paus`

Pomodoro is too rigid. `paus` is a [Third Time](https://www.lesswrong.com/posts/RWu8eZqbwgB9zaerh/third-time-a-better-way-to-work) stopwatch. It allows you to work as long as you want, earn break time, and spend it whenever.

`paus` runs as a background daemon. Control it from any terminal or status bar.

## How it works

1. **Start focusing:** run `paus focus`. The clock runs.
2. **Take a break when you want:** run `paus break`. Every minute of focus earns 1/3 of a minute off (by default).
3. **Check your balance:** Run `paus status`. Your balance tracks what you've earned vs. spent. Positive balance means break time remaining. Negative means you've gone over.

## Install

### Nix (recommended)

The flake includes a Home Manager module that installs `paus` and starts the daemon automatically on login:

```nix
# flake.nix
inputs.paus.url = "github:belovedEmperor/paus";

# home.nix
imports = [ inputs.paus.homeManagerModules.default ];
```

### Cargo

```sh
cargo install --path .
```

## Commands

```sh
paus daemon run     # start the background daemon
paus focus          # start focusing
paus break          # start a break
paus toggle-phase   # switch between focus and break
paus pause          # pause the stopwatch
paus unpause        # resume
paus toggle-pause   # toggle pause state
paus status         # show phase, balance, and pause state
paus daemon stop    # stop the daemon
```

## Status output

```
⏰ 12:34 ⚖️ 04:11 ▶    # focusing: focus time + balance remaining
🏖️ 05:00 ⚖️ -00:49 ▶  # on a break: break time + balance (negative = over)
⏰ 12:34 ⚖️ 04:11 ⏸   # paused
```

Pin specific fields with flags:

```sh
paus status --focus    # always show focus time
paus status --breaks   # always show break time
paus status --balance  # always show balance
```

## Break ratios

The default ratio is Standard (1/3): 30 minutes of focus earns 10 minutes of break.

| Name        | Ratio | Break earned per 30 min |
|-------------|-------|-------------------------|
| Lazy        | 1/2   | 15 min                  |
| Standard    | 1/3   | 10 min                  |
| Industrious | 1/4   | 7.5 min                 |
| Hard        | 1/5   | 6 min                   |
| Grinding    | 1/6   | 5 min                   |

## State

Saved to `~/.local/share/paus/state.json`. On daemon restart, totals carry over but the phase timer resets, so time while the daemon was down is not counted.

## TODO
- [ ] Add config file support.
- [ ] Add option to change break ratio.
- [ ] Add reset state `phase` on start.
- [ ] Add reset state times daily.
- [ ] Rename status `*_seconds` fields since it could be minute.s
- [ ] Add save tracked times, and check saved times.
- [ ] Add more bar support.
