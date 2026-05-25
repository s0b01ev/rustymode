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

use std::io::Cursor;
use async_trait::async_trait;
use slack_morphism::{
    prelude::*
};
use image::{ImageReader, ImageFormat};

use crate::{error::ErrorKind, ChatPayload, Messenger };

type SlackSession<'a> = SlackClientSession<'a, SlackClientHyperHttpsConnector>;

pub struct SlackCtx {
    client: SlackClient<SlackClientHyperHttpsConnector>,
    token: SlackApiToken,
    pub channel: SlackChannelId,
}

impl SlackCtx {
    pub fn new(channel: SlackChannelId, token: SlackApiToken) -> std::io::Result<Self> {
        let connector = SlackClientHyperConnector::new()?;
        let client = SlackClient::new(connector);

        Ok(Self { client, token, channel })
    }

    pub fn session(&self) -> SlackSession<'_> {
        self.client.open_session(&self.token)
    }
}

pub struct SlackMessenger {
    ctx: SlackCtx,
}

impl SlackMessenger {
    pub fn new(channel: SlackChannelId, token: SlackApiToken) -> std::io::Result<Self> {
        Ok(Self {
            ctx: SlackCtx::new(channel, token)?,
        })
    }
}

#[async_trait]
impl Messenger for SlackMessenger {
    async fn send(&self, payload: &ChatPayload) -> anyhow::Result<()> {
        let session = self.ctx.session();

        match payload {
            ChatPayload::Text { text } => {
                let req =
                    SlackApiChatPostMessageRequest::new(
                        self.ctx.channel.clone(),
                        SlackMessageContent::new().with_text(text.into())
                    );
                session.chat_post_message(&req).await?;
            }

            ChatPayload::TextWithEmoji { text, emoji } => {
                let body = format!(":{}: {}", emoji, text);

                let req =
                    SlackApiChatPostMessageRequest::new(self.ctx.channel.clone(),
                                                        SlackMessageContent::new().with_text(body.into())
                    );

                session.chat_post_message(&req).await?;
            }

            ChatPayload::Image { img } => {
                let get_upload_url_req =
                    SlackApiFilesGetUploadUrlExternalRequest::new("image.png".into(), img.len());
                let upload_url_resp = session.get_upload_url_external(&get_upload_url_req).await?;

                let file_upload_req = SlackApiFilesUploadViaUrlRequest::new(
                    upload_url_resp.upload_url,
                    (*img.clone()).to_owned(),
                    "image/png".into(),
                );

                let file_upload_resp = session.files_upload_via_url(&file_upload_req).await?;

                let complete_file_upload_req =
                    SlackApiFilesCompleteUploadExternalRequest::new(vec![SlackApiFilesComplete::new(
                        upload_url_resp.file_id,
                    )])
                        .with_channel_id("C06KDAZ3EBE".into());

                let complete_file_upload_resp = session
                    .files_complete_upload_external(&complete_file_upload_req)
                    .await?;
            }
        }

        Ok(())
    }
}