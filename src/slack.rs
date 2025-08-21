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


extern crate slack_hook;
use slack_hook::{Slack, PayloadBuilder, Payload};

use crate::{error::ErrorKind, Messenger};

pub struct SlackMessenger {
    pub slack: Slack,
    pub channel: String,
    pub username: String,
}
pub fn new(slack_url: &str, slack_channel: &str, slack_user: &str) -> Result<SlackMessenger, ErrorKind> {
   let slack = Slack::new(slack_url)
       .map(|s| SlackMessenger{
           slack: s,
           channel: slack_channel.to_string(),
           username: slack_user.to_string(),
       });
   match slack {
        Ok(slack) => Ok(slack),
        Err(e) => Err(ErrorKind::CreateSlackClientErr(e.to_string())),
   }
}

impl Messenger for SlackMessenger {
    fn send(&mut self, payload: Payload) -> Result<(), ErrorKind> {
        let res = &self.slack.send(&payload);
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(ErrorKind::UnableToSendSlackMessage(e.to_string())),
        }
    }

   fn payload(&self, text: String) -> Result<Payload, ErrorKind> {
        let payload = PayloadBuilder::new()
            .text(text)
            .channel(&self.channel)
            .username(&self.username)
            .build();
        match payload {
            Ok(payload) => Ok(payload),
            Err(_) => Err(ErrorKind::CreateSlackPayloadErr),
        }
    }
}