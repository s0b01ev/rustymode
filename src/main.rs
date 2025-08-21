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

#[cfg(test)]
mod test;

use rustymode::{args::{Args, Parser}, color::{Colorizer, MsgType}, config::Config, Codec, Grabber, MotionDetector, Writer, VideoStreamer, Messenger, slack, Frame};
use chrono::Local;
use signal_hook::{consts::SIGINT, flag::register};
use std::io;
use std::{
    path::Path,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
};
use std::io::Write;
use std::net::TcpListener;
use std::os::unix::raw::time_t;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use opencv::core::{Mat, Vector};
use opencv::imgcodecs;
use opencv::videoio::{CAP_ANY, VideoCapture, VideoCaptureTrait};

fn main() -> io::Result<()> {
    // Parse CLI arguments.
    let args = Args::parse();
    // Parse config file and override options with CLI arguments.
    let config = match Config::parse() {
        Ok(config) => config,
        Err(e) => {
            Colorizer::new(MsgType::Error, args.no_color, "error [config]", e).print()?;
            Colorizer::new(
                MsgType::Warn,
                args.no_color,
                "warning",
                "using default configuration",
            )
            .print()?;
            Config::default()
        }
    }
    .override_with_args(args);

    // Format video file path as <config.directory/date&time>.
    let filename = Local::now()
        .format(
            config
                .directory
                // Output video file name (derived by file format) + extension.
                .join(Path::new(&config.format).with_extension("mkv"))
                // Convert Path object to string.
                .to_str()
                .unwrap(),
        )
        .to_string();

    // Instance of the frame grabber.
    let grabber = match &config.video {
        // VideoCapture is video file.
        Some(video) => Grabber::from_file(video),
        // VideoCapture is live camera.
        None => Grabber::new(
            config.index.into(),
            config.height.into(),
            config.width.into(),
            config.framerate.into(),
        ),
    };
    let grabber = match grabber {
        Ok(grabber) => grabber,
        Err(e) => {
            Colorizer::new(MsgType::Error, config.no_color, "error", e).print()?;
            process::exit(1);
        }
    };

    // Print info.
    if !config.quiet {
        let mut colorizer = Colorizer::empty(MsgType::Info, config.no_color);

        let input = if let Some(video) = &config.video {
            video.display().to_string()
        } else {
            //format!("/dev/video{}", &config.index)
            format!("{}", &config.index)
        };

        let messages = vec![
            ("==> Input", input),
            ("==> Framerate", grabber.get_fps().to_string()),
            (
                "==> Frame size",
                format!("{}x{}", grabber.get_width(), grabber.get_height()),
            ),
            ("==> Printing overlay", format!("{}", config.overlay)),
            ("==> Output video file", filename.clone()),
        ];

        for msg in messages {
            colorizer.update(msg.0, msg.1);
            colorizer.print()?;
        }
    }

    // Instance of the motion detector.
    let detector = MotionDetector::new();

    // Instance of the frame writer.
    let writer = match Writer::new(
        &filename,
        Codec::XVID,
        grabber.get_fps(),
        grabber.get_size(),
        config.overlay,
        config.overlay_border,
    ) {
        Ok(writer) => writer,
        Err(e) => {
            Colorizer::new(MsgType::Error, config.no_color, "error", e).print()?;
            process::exit(1);
        }
    };

   // Instance of the video streamer.
    let streamer = match VideoStreamer::new(
        config.index.into(),
        config.height.into(),
        config.width.into(),
        config.framerate.into(),
        config.streamer_listener.as_str(),
        config.streamer_image_encode.as_str(),
    ) {
        Ok(streamer) => streamer,
        Err(e) => {
            Colorizer::new(MsgType::Error, config.no_color, "error", e).print()?;
            process::exit(1);
        }
    };

    let messenger = match slack::new(
        config.slack_url.as_str(),
        config.slack_channel.as_str(),
        config.slack_user.as_str(),
    ) {
        Ok(messenger) => messenger,
        Err(e) => {
        Colorizer::new(MsgType::Error, config.no_color, "error", e).print()?;
        process::exit(1);
    }
    };

    // Save memory dropping `filename`.
    drop(filename);

    // Run the program.
    run(grabber, detector, writer, streamer, Box::new(messenger) as Box<dyn Messenger + Send>, config.no_color)?;

    // Gracefully terminated execution.
    if !config.quiet {
        Colorizer::new(MsgType::Info, config.no_color, "\nbombuscv", "done!").print()?;
    }

    Ok(())
}

/// Run `bombuscv`: spawn & join frame grabber, detector and writer threads.
fn run(
    mut grabber: Grabber,
    mut detector: MotionDetector,
    mut writer: Writer,
    mut streamer: VideoStreamer,
    mut messenger: Box<dyn Messenger + Send>,
    no_color: bool,
) -> io::Result<()> {
    // Create channels for message passing between threads.
    // NOTE: using mpsc::sync_channel (blocking) to avoid channel size
    // growing indefinitely, resulting in infinite memory usage.
    let (raw_tx, raw_rx) = mpsc::sync_channel(100);
    let (proc_tx, proc_rx) = mpsc::sync_channel(100);
    let (dtr_tx, msgr_rx) = mpsc::sync_channel(100);
    let (streamer_tx, streamer_rx) = mpsc::sync_channel(100);

    let streaming_enabled = Arc::new(AtomicBool::new(false));
    let grabber_flag = streaming_enabled.clone();
    let streamer_flag = streaming_enabled.clone();

    let term = Arc::new(AtomicBool::new(false));
    let term_grabber = Arc::clone(&term);
    let term_streamer = Arc::clone(&term);
    let term_writer = Arc::clone(&term);
    let term_detector = Arc::clone(&term);
    let term_messenger = Arc::clone(&term);

    // Register signal hook for SIGINT events: in this case error is unrecoverable, so report
    // it to the user & exit process with code error code.
    if let Err(e) = register(SIGINT, Arc::clone(&term)) {
        Colorizer::new(
            MsgType::Error,
            no_color,
            "fatal error",
            format!("unable to register SIGINT hook '{e}'"),
        )
            .print()?;
        process::exit(1);
    };

    // Spawn frame grabber thread:
    // this thread captures frames and passes them to the motion detecting thread.
    let grabber_handle = thread::spawn(move || -> io::Result<()> {

        // Start grabber loop: loop guard is 'received SIGINT'.
        while !term_grabber.load(Ordering::Relaxed) {
            let frame = match grabber.grab() {
                Ok(frame) => frame,
                Err(e) => {
                    Colorizer::new(MsgType::Warn, no_color, "warning", e).print()?;
                    continue;
                }
            };


            let frame_clone = Frame{ frame: frame.frame.clone(), datetime: frame.datetime.clone() };
            // Grab frame and send it to the motion detection thread.
            if raw_tx.send(frame).is_err() {
                break;
            }

            if grabber_flag.load(Ordering::Relaxed) {
                // Grab frame clone and send it to the video streamer thread.
                if streamer_tx.send(frame_clone).is_err() {
                    break;
                }
            }
        }
        Ok(())
    });

    // Spawn motion detection thread:
    // this thread receives frames from the grabber thread, processes it and if motion is detected,
    // passes the frame to the frame writing thread.
    //let mut message_last_sent = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let mut message_last_sent = Duration::from_secs(0);
    let detector_handle = thread::spawn(move || -> io::Result<()> {
        // Loop over received frames from the frame grabber.
        for frame in raw_rx {
            if term_detector.load(Ordering::Relaxed) {
                return Ok(());
            }
            match detector.detect_motion(frame) {
                // Valid frame is received.
                Ok(val) => {
                    // Motion has been detected: send frame to the video writer.
                    if let Some(frame) = val {
                        if proc_tx.send(frame).is_err() {
                            Colorizer::new(
                                MsgType::Warn,
                                no_color,
                                "warning",
                                "unable to send processed frame to video output",
                            )
                            .print()?;
                        };
                        // TODO: make it sending a frame with motion detected rather than just bool
                        if dtr_tx.send(true).is_err() {
                            let time_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                            if time_now - message_last_sent > Duration::from_secs(10) {
                                message_last_sent = time_now;
                                Colorizer::new(
                                    MsgType::Warn,
                                    no_color,
                                    "warning",
                                    "unable to send signal to messenger thread",
                                )
                                .print()?;
                            }
                        };
                    }
                }
                // Last captured frame was an empty frame: no more input is provided, interrupt the
                // thread (break the loop).
                Err(_) => break,
            }
        }

        Ok(())
    });

    // Spawn frame writer thread:
    // this thread receives the processed frames by the motion detecting thread and writes them in
    // the output video output.
    let writer_handle = thread::spawn(move || -> io::Result<()> {
        if term_writer.load(Ordering::Relaxed) {
            return Ok(());
        }
        // Loop over received frames from the motion detector.
        for frame in proc_rx {
            // Write processed frames (motion detected) to the video output.
            if let Err(e) = writer.write(frame) {
                Colorizer::new(MsgType::Warn, no_color, "warning", e).print()?;
            };
        }

        Ok(())
    });

    // spawn video streaming thread
    // this thread receives frames from
    let streamer_handle = thread::spawn(move || -> io::Result<()> {
        let mut buf = Vector::new();

        let response = "HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=frame\r\n\r\n".to_string();

        while !term_streamer.load(Ordering::Relaxed) {
            streamer_flag.store(false, Ordering::Relaxed);
            streamer.listener.set_nonblocking(true).unwrap();
            match streamer.listener.accept() {
                Ok((mut stream, addr)) => {
                    let client_connected_msg= Local::now().format("%Y-%m-%d_%H-%M-%S").to_string() + " HTTP Client Connected from " + addr.to_string().as_str();
                    Colorizer::new(MsgType::Info, no_color, "==>", client_connected_msg).print()?;

                    streamer_flag.store(true, Ordering::Relaxed);

                    match stream.write_all(response.as_bytes()) {
                        Ok(_) => (),
                        Err(e) => {
                            eprintln!("Client disconnected or write error: {}", e);
                        }
                    }

                    for frame in streamer_rx.iter() {
                        if term_streamer.load(Ordering::Relaxed) {
                            return Ok(());
                        }
                        buf.clear();
                        let _ = imgcodecs::imencode(".jpg", &frame.frame, &mut buf, &Vector::new());

                        let image_data = format!(
                            "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                            buf.len()
                        );

                        match stream.write_all(image_data.as_bytes()) {
                            Ok(_) => (),
                            Err(e) => {
                                eprintln!("Client disconnected or write error: {}", e);
                                streamer_flag.store(false, Ordering::Relaxed);
                                break
                            }
                        }
                        match stream.write_all(buf.as_slice()) {
                            Ok(_) => (),
                            Err(e) => {
                                eprintln!("Client disconnected or write error: {}", e);
                                streamer_flag.store(false, Ordering::Relaxed);
                                break
                            }
                        }
                        match stream.write_all(b"\r\n") {
                            Ok(_) => (),
                            Err(e) => {
                                eprintln!("Client disconnected or write error: {}", e);
                                streamer_flag.store(false, Ordering::Relaxed);
                                break
                            }
                        }
                        match stream.flush() {
                            Ok(_) => (),
                            Err(e) => {
                                eprintln!("Client disconnected or write error: {}", e);
                                streamer_flag.store(false, Ordering::Relaxed);
                                break
                            }
                        }
                    }
                },
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // No pending connections, sleep a bit
                    thread::sleep(Duration::from_millis(100));
                },
                Err(e) => {
                    eprintln!("accept() error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    });

    // Spawn messenger thread:
    // this thread receives a message from detector thread, if motion detected
    let messenger_handle = thread::spawn(move || -> io::Result<()> {
        // Loop over received frames from the motion detector.
        for detected in msgr_rx {
            if term_messenger.load(Ordering::Relaxed) {
                println!("Exit 0 from messenger thread");
                return Ok(());
            }
            let time_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let motion_detected_msg= Local::now().format("%Y-%m-%d_%H-%M-%S").to_string() + " Motion Detected";
            if time_now - message_last_sent > Duration::from_secs(5) {
                message_last_sent = time_now;
                Colorizer::new(MsgType::Info, no_color, "==>", motion_detected_msg.clone()).print()?;
                let payload = messenger.payload(motion_detected_msg.to_owned())
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                let res = messenger.send(payload);
                match res {
                    Ok(()) => (),
                    Err(x) => println!("ERR: {:?}",x)
                }
            }
        }

        println!("Exit 0 from messenger thread");
        Ok(())
    });

    // Join all threads.
    grabber_handle.join().expect("cannot join grabber thread")?;
    detector_handle .join() .expect("cannot join detector thread")?;
    writer_handle.join().expect("cannot join writer thread")?;
    streamer_handle.join().expect("cannot join streamer thread")?;
    messenger_handle.join().expect("cannot join messenger thread")?;

    Ok(())
}
