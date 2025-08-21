// rustymode: Fork of bombuscv, originally an OpenCV-based motion detection/recording software built for research on bumblebees.
// Originally developed as bombuscv by Marco Radocchia (C) 2022
// Modified and renamed to rustymode by Dmitry Sobolev (C) 2025
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see https://www.gnu.org/licenses/.
//
//!
//! # rustymode
//!
//! Motion Detection, Video Streaming and Alerting with Rust.


use crate::{Codec, Config, Grabber, Local, MotionDetector, Path, Writer};
use bombuscv_rs::Frame;
use directories::BaseDirs;
use std::{fs, time::Instant};

#[test]
fn sync_frame_processing_avg_time() {
    // Number of frames to acquire.
    const N: usize = 2000;

    // Generate the absolute path for HOME.
    let home = BaseDirs::new().unwrap().home_dir().to_path_buf();

    // Parse CLI arguments.
    let config = Config {
        index: 0,
        height: 1080,
        width: 1920,
        framerate: 60,
        video: Some(home.join("test.mkv")),
        directory: home,
        format: String::from("output"),
        overlay: false,
        no_color: true,
        quiet: false,
        overlay_border: 2,
        slack_url: "".to_string(),
        slack_channel: "#cam".to_string(),
        slack_user: "detector".to_string(),
        streamer_image_encode: ".jpeg".to_string(),
        streamer_listener: "127.0.0.1:8740".to_string(),
    };

    // Format video file path as <config.directory/date&time>.
    let filename = Local::now()
        .format(
            config
                // Output video file directory.
                .directory
                // Output video file name (derived by file format) + extension.
                .join(Path::new(&config.format).with_extension("mkv"))
                // Convert Path object to string;
                .to_str()
                .unwrap(),
        )
        .to_string();

    // Vector of frames to test performance on.
    let mut frames: Vec<Frame> = Vec::with_capacity(N);
    let mut detected_frames = 0;

    // Instance of the frame grabber.
    let mut grabber = match &config.video {
        // VideoCapture is video file.
        Some(video) => Grabber::from_file(video),
        // VideoCapture is live camera.
        None => Grabber::new(
            config.index.into(),
            config.height.into(),
            config.width.into(),
            config.framerate.into(),
        ),
    }
    .unwrap();

    // Instance of the motion detector.
    let mut detector = MotionDetector::new();

    // Instance of the frame writer.
    let mut writer = Writer::new(
        &filename,
        Codec::XVID,
        grabber.get_fps(),
        grabber.get_size(),
        config.overlay,
        config.overlay_border,
    )
    .unwrap();

    // Print config options if config.quiet is false.
    if !config.quiet {
        println!("{:#?}", &config);
    }

    // Acquire N frames.
    for _ in 0..N {
        frames.push(grabber.grab().unwrap());
    }

    // Save the start time.
    let start = Instant::now();
    for frame in frames {
        match detector.detect_motion(frame) {
            Ok(frame) => {
                if let Some(frame) = frame {
                    // If frame is detected, write it to the file.
                    writer.write(frame).unwrap();
                    // Count the detected frames.
                    detected_frames += 1;
                }
            }
            Err(_) => panic!("not enaugh frames to run the test!"),
        }
    }

    //Calculate the elapsed time to process motion detection on all the N frmaes.
    let tot_dur_ns = start.elapsed();
    let dur_ns = tot_dur_ns.div_f32(N as f32);
    let max = 1e3 / config.framerate as f32;
    println!("==> # saved frames: {}", detected_frames);
    println!("==> processing motion detection took: {:?}", tot_dur_ns);
    println!(
        "==> processing motion detection per frame took (avg): {:?}",
        dur_ns
    );
    println!("==> max value allowed: {}ms", max);

    // Remove generated video file.
    fs::remove_file(filename).expect("unable to remove output file.");

    assert!(dur_ns.subsec_micros() <= (max * 1e3) as u32);
}
