// src/llm/llm.rs

use tokio::net::UnixStream;
use async_stream::stream;
use futures_util::stream::{Stream, StreamExt};

use crate::llm::llm_events::{
    LlmEvent,
    LlmError,
    GenerationOutput,
    EndEvent,
};
use crate::llm::protocol::{
    FrameType,
    write_frame,
    read_frame
};

const MAX_OUTPUT_SIZE:      usize    = 512 * 1024;

pub struct LLM {
    socket_path: String,
}

impl LLM {
    pub fn new(socket_path: &str) -> Self {
        Self { socket_path: socket_path.to_string() }
    }

    pub fn stream_generate<'a>(
        &'a self,
        prompt: &'a str
    ) -> impl Stream<
        Item = Result<LlmEvent, LlmError>
    > + 'a {
        stream! {
            println!("[LLM] connecting to socket {}...", self.socket_path);

            // connect to socket
            let mut socket = match UnixStream::connect(&self.socket_path).await {
                Ok(socket) => socket,

                Err(error) => {
                    yield Err(
                        LlmError::Connect(error)
                    );
                    return;
                }
            };
            
            println!("[LLM] connected to socket");

            // -------------------------
            // Send REQUEST frame
            // -------------------------

            if let Err(error) = write_frame(
                &mut socket,
                FrameType::Request,
                prompt.as_bytes(),
            )
            .await {
                yield Err(
                    LlmError::Protocol(error)
                );
                return;
            }

            // -------------------------
            // Read response frames
            // -------------------------

            let mut utf8_buffer = Vec::new();
            let mut output_size = 0usize;

            loop {
                let frame = match read_frame(&mut socket).await {
                    Ok(frame) => frame,

                    Err(error) => {
                        yield Err(
                            LlmError::Protocol(error)
                        );
                        return;
                    }
                };

                match frame.kind {

                    FrameType::Content => {
                        output_size += frame.payload.len();

                        if output_size > MAX_OUTPUT_SIZE {
                            yield Err(
                                LlmError::OutputTooLarge
                            );
                            return;
                        }

                        utf8_buffer.extend_from_slice(&frame.payload);

                        if std::str::from_utf8(&utf8_buffer).is_ok() {

                            let content = match String::from_utf8(std::mem::take(&mut utf8_buffer)) {
                                Ok(content) => content,

                                Err(error) => {
                                    yield Err(
                                        LlmError::InvalidUtf8(error)
                                    );
                                    return;
                                }
                            };

                            yield Ok(
                                LlmEvent::Content(content)
                            );
                        }
                    }

                    FrameType::End => {
                        let end: EndEvent = match serde_json::from_slice::<EndEvent>(&frame.payload) {
                            Ok(end) => end,

                            Err(error) => {
                                yield Err(
                                    LlmError::InvalidControlFrame(error.to_string())
                                );
                                return;
                            }
                        };
                        
                        yield Ok(
                            LlmEvent::End(end.reason)
                        );
                        return;
                    }

                    FrameType::Error => {
                        let message = String::from_utf8_lossy(&frame.payload);

                        yield Err(
                            LlmError::Remote(message.into_owned())
                        );
                        return;
                    }

                    FrameType::Status => {
                        /*
                        * Reserved for broker:
                        * queued / running / ...
                        */
                    }

                    FrameType::Request | FrameType::Cancel => {
                        yield Err(
                            LlmError::InvalidControlFrame(
                                format!("unexpected frame from server: {:?}",frame.kind)
                            )
                        );

                        return;
                    }
                }
            }            
        }
    }

    // for non streaming generation:
    pub async fn generate(
        &self,
        prompt: &str,
    ) -> Result<GenerationOutput, LlmError> {

        let stream = self.stream_generate(prompt);

        futures_util::pin_mut!(stream);

        let mut output: String = String::new();

        while let Some(event) = stream.next().await {
            match event? {
                LlmEvent::Content(content) => {
                    output.push_str(&content);
                }

                LlmEvent::End(reason) => {
                    return Ok(
                        GenerationOutput { output, reason }
                    );
                }
            }
        }

        Err(
            LlmError::UnexpectedEof
        )
    }
    
}
