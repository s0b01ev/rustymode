<div align="center">
  <h1 align="center">MotionDetector</h1>

  ![GitHub releases](https://img.shields.io/github/downloads/s0b01ev/rustymode/total?color=%23a9b665&logo=github)
  ![GitHub source size](https://img.shields.io/github/languages/code-size/s0b01ev/rustymode?color=ea6962&logo=github)
  ![GitHub open issues](https://img.shields.io/github/issues-raw/s0b01ev/rustymode?color=%23d8a657&logo=github)
  ![GitHub open pull requests](https://img.shields.io/github/issues-pr-raw/s0b01ev/rustymode?color=%2389b482&logo=github)
  ![GitHub sponsors](https://img.shields.io/github/sponsors/s0b01ev?color=%23d3869b&logo=github)
  ![Crates.io downloads](https://img.shields.io/crates/d/rustymode?label=crates.io%20downloads&color=%23a9b665&logo=rust)
  ![Crates.io version](https://img.shields.io/crates/v/rustymode?logo=rust&color=%23d8a657)
  ![GitHub license](https://img.shields.io/github/license/s0b01ev/rustymode?color=%23e78a4e)
</div>

Motion detection & video recording software based on OpenCV. This project was created as an extension of the
[BombusCV](https://github.com/marcoradocchia/bombuscv) project authored  by [Marco Radocchia](https://github.com/marcoradocchia) in attempt to create a more generic motion detection
software and for educational purpose.

The following changes were made to the original project:
- Added support for video streaming
- Added support for Slack alerting
- OpenCV version updated to 4.9.0

More advanced alerting and support for other popular messengers will be added in the future. 

As a separate note want to thank the original author of the project [Marco Radocchia](https://github.com/marcoradocchia).
This thing works great, and I really enjoyed the code he wrote! Hope to give the project some love.

## Index

- [Use case](#use-case)
- [Examples](#examples)
- [Install](#install)
  - [Requirements](#requirements)
  - [Cargo](#cargo)
  - [Install on RaspberryPi 4](#install-on-raspberrypi-4)
- [Usage](#usage)
- [Configuration](#configuration)
- [Changelog](#changelog)
- [ToDo](#todo)
- [Chat Support](#chat-support)
- [License](#license)

## Use case

I decided to add video streaming and alerting to the original project to be able to use it as a general purpose motion
detector. And of course to learn Rust and have fun!. It has been used
with a
[Raspberry Pi 4](https://www.raspberrypi.com/products/raspberry-pi-4-model-b/)[^1]
and a
[Raspberry Pi HQ Camera](https://www.raspberrypi.com/products/raspberry-pi-high-quality-camera/)[^2]

`rustymode` offers realtime motion detection & video recording[^3] using
camera input and can be directly used on fieldwork. However, using the `video`
option, live camera input can be replaced with a pre-recorded video file: this
is useful to _remove dead moments_ from videos and reduce/remove the need of
manual video trimming. Video stream can be viewed using a web browser.
Slack alerting can be configured to receive notifications when motion is
detected.


[^1]: 4GB of RAM memory, powered by a 30000mAh battery power supply, which
  means this setup can be also reproduced in locations where no AC is available
[^2]: 12.3 megapixel _Sony IMX477_ sensor
[^3]: Based on hardware (RasberryPi 4 at the moment of writing can handle
  640x480 resolution at 60fps)

## Install

For installation on *RaspberryPi* check [Install on RaspberryPi
4](#install-on-raspberrypi-4).

### Requirements

This program requires a working installation of **OpenCV** (`>=4.5.5`).
Building OpenCV from source is recommended (if you're going to build OpenCV
from source make sure to also install OpenCV dependencies), although it should
work with precompiled packages in your distro's repositories (it has been
tested with success on *ArchLinux* with the `extra/opencv` package).

### Cargo

[^4]: Assuming Rust installed

### Install on RaspberryPi 4

It is strongly recommended to use a RaspberryPi 4 with at least 4GB of RAM.
Also, before trying to install, please enable *Legacy Camera* support under
*Interface options*  running `raspi-config` and reboot. Since installation on a
RaspberryPi may be a little bit *tricky*, an installation script is
provided[^5]. It takes care of updating & preparing the system, compiling
*OpenCV* and installing *Rustup* and finally **BombusCV**. You can run the
[instllation script](rustymode-raspi.sh) using `curl`:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/s0b01ev/rustymode/master/rustymode-raspi.sh | sh
```

[^5]: RaspberryPi OS 64 bits required in order to install using the script

## Usage

```
rustymode 0.3.0
Marco Radocchia <marco.radocchia@outlook.com>
OpenCV based motion detection/recording/streaming software with alerting support

USAGE:
    rustymode [OPTIONS]

OPTIONS:
    -d, --directory <DIRECTORY>    Output video directory
    -f, --framerate <FRAMERATE>    Video capture framerate
        --format <FORMAT>          Output video filename format (see
                                   <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>
                                   for valid specifiers)
    -h, --help                     Print help information
    -H, --height <HEIGHT>          Video capture frame height
    -i, --index <INDEX>            /dev/video<INDEX> capture camera index
        --no-color                 Disable colored output
    -o, --overlay                  Date&Time video overlay
    -q, --quiet                    Mute standard output
    -v, --video <VIDEO>            Video file as input
    -V, --version                  Print version information
    -W, --width <WIDTH>            Video capture frame width
```

Specifying `width`, `height` & `framerate` will make `bombuscv` probe the
capture device for the closest combination of values it can provide and select
them. In other words: if you required valid options, they will be used,
otherwhise `rustymode` will adapt those to the closest available combination[^6].

Note that `video` option, which runs `rustymode` with a pre-recorded video
input, is incompatible with `framerate`, `width`, `height` and `overlay`. Also,
if these options are specified in the configuration file, they are going to be
ignored. This because the first two are auto-detected from the input file while
the last makes no sense if used with a non-live video feed; same rules apply to
CLI arguments.

[^6]: Same rules apply to configuration file

## Configuration

All CLI options (except `video` and `no-color`) can be set in a *optional* configuration file
stored at `$XDG_CONFIG_HOME/rustymode/config.toml` by default or at any other
location in the filesystem specified by setting `RUSTYMODE_CONFIG` environment
variable. CLI options/arguments/flags override those defined in the
configuration file. Below listed an example configuration file:
```toml
# be quiet (mute stdout)
quiet = false
# output video directory
directory = "~/output_directory/"
# output video filename format (see
# https://docs.rs/chrono/latest/chrono/format/strftime/index.html for valid specifiers)
format = "%Y-%m-%dT%H:%M:%S"

# The following options are ignored if bombuscv is run with `--video` option
# /dev/video<index> camera input
index = 0
# video capture frame width
width = 640
# video capture frame height
height = 480
# video capture framerate
framerate = 30
# date&time video overlay
overlay = true
# date&time video overlay border
overlay_border = 2
# Slack web hook URL
slack_url = "https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX"
# Slack channel
slack_channel  = "#cam"
# Slack user
slack_user = "detector"
# Streamer listener
streamer_listener = "0.0.0.0:8740"
# Streamer encoder image type
streamer_image_encode = ".jpg"
```

## Changelog

Complete [CHANGELOG](CHANGELOG.md).

## ToDo

- [x] Provide build & install instructions in [README](README.md), as well as
  the instructions to install OpenCV.
- [x] Make install script for automated installation on RaspberryPi.
- [x] Passing `video` or `directory` options in the configuration file using
  `~/<path>` results in an error: in the Deserialize expanding `~` to
  absolute path is required.
- [x] Using `video`, _date&time_ overlay generated on frame grabbed makes no
  sense: disable video overlay while using `video` option.
- [x] Add option to specify custom config path using env variables.
- [x] Add option to specify (in config file or via CLI argument) a custom
  output video filename formatter (must be [chrono DateTime
  syntax](https://docs.rs/chrono/latest/chrono/format/strftime/index.html)).
- [x] Add thread signalling to interrupt grabber thread and gracefully
  terminate the execution.
- [x] Move logic from `main` to newly defined `run`.


## License

[GPLv3](LICENSE)
