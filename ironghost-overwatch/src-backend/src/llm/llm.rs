// src/llm/llm.rs

use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use async_stream::stream;
use futures_util::stream::Stream;
//use std::time::Duration;
use std::io;

use crate::llm::llm_events::{
    LlmEvent,
    LlmError,
    GenerationStopReason,
    EndEvent,
};

const MAX_FRAME_SIZE:       usize    = 1024 * 1024;
const MAX_OUTPUT_SIZE:      usize    = 512 * 1024;
//const GENERATION_TIMEOUT:   Duration = Duration::from_secs(120);

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
            let mut socket = match UnixStream::connect(&self.socket_path)
                .await {
                    Ok(socket) => socket,

                    Err(error) => {
                        yield Err(
                            LlmError::Connect(error)
                        );
                        return;
                    }
                };
            
            println!("[LLM] connected to socket");

            // --- Send prompt ---
            if let Err(error) = send_prompt(&mut socket, prompt)
                .await {
                    yield Err(error);
                    return;
                }
                
            // --- Read stream ---
            let mut utf8_buffer = Vec::new();
            let mut output_size = 0usize;

            loop {
                let frame = match read_frame(&mut socket)
                    .await {
                        Ok(frame) => frame,

                        Err(error) => {
                            yield Err(error);
                            return;
                        }
                    };

                // end of stream
                if let Some( reason ) = parse_end_event( &frame ) {
                    if !utf8_buffer.is_empty() {
                        match String::from_utf8( std::mem::take( &mut utf8_buffer ) ) {
                            Ok(content) => {
                                yield Ok( LlmEvent::Content( content ) );
                            }

                            Err(error) => {
                                yield Err( LlmError::InvalidUtf8( error ) );
                                return;
                            }
                        }
                    }

                    yield Ok( LlmEvent::End( reason ) );

                    println!("\n\n[LLM] end of stream!");
                    
                    return;
                }

                output_size += frame.len();

                if output_size > MAX_OUTPUT_SIZE {
                    yield Err( LlmError::OutputTooLarge );
                    return;
                }

                // accumulate UTF-8 fragments
                utf8_buffer.extend_from_slice(&frame);

                if std::str::from_utf8( &utf8_buffer ).is_ok() {
                    let bytes = std::mem::take( &mut utf8_buffer );

                    match String::from_utf8(bytes) {
                        Ok(content) => {
                            yield Ok( LlmEvent::Content( content ) );
                        }

                        Err(error) => {
                            yield Err( LlmError::InvalidUtf8( error ) );
                            return;
                        }
                    }
                }
            }            
        }
    }

    // for non streaming generation:
    pub async fn generate(&self, prompt: &str) -> String {

        println!("[LLM] connecting to socket {}...", self.socket_path);

        // connect to socket
        let mut stream: UnixStream = UnixStream::connect(&self.socket_path)
            .await
            .expect("Unable to connect");

        println!("[LLM] connected to socket");

        // --- Send prompt ---
        // send length and data
        let data     : &[u8]   = prompt.as_bytes();
        let len_bytes: [u8; 4] = (data.len() as u32).to_le_bytes();

        stream.write_all(&len_bytes).await.unwrap(); println!("[LLM] length sent: {}", data.len());
        stream.write_all(data).await.unwrap();       println!("[LLM] data sent: {}",   prompt    );

        // --- Read stream ---
        // read tokens till END_OF_STREAM
        let mut utf8_buffer : Vec<u8>  = Vec::new();
        let mut final_output: String   = String::new();

        loop {
            // read length of token
            let mut len_buf: [u8; 4] = [0u8; 4];
            if stream.read_exact(&mut len_buf).await.is_err() {
                println!("[LLM] stream len not exact! break!");
                break;
            }
            let len: usize = u32::from_le_bytes(len_buf) as usize;

            // read token
            let mut token_buf: Vec<u8> = vec![0u8; len];
            stream.read_exact(&mut token_buf).await.unwrap();

            // end of stream
            if token_buf == b"\n=== END_OF_STREAM ===" {
                println!("\n\n[LLM] end of stream! break!");
                break;
            }

            // accumulate UTF-8 fragments
            utf8_buffer.extend_from_slice(&token_buf);

            if let Ok(s) = std::str::from_utf8(&utf8_buffer) {
                final_output.push_str(s);
                print!("{}", s);
                utf8_buffer.clear();
            }
        }

        final_output
    }
    
}

async fn send_prompt(
    stream: &mut UnixStream,
    prompt: &str,
) -> Result<(), LlmError> {
    let data: &[u8] = prompt.as_bytes();

    let len: u32 = u32::try_from(data.len())
        .map_err(|_| LlmError::FrameTooLarge {
            size: data.len(),
        })?;

    stream
        .write_all(&len.to_le_bytes())
        .await
        .map_err(LlmError::Write)?;

    stream
        .write_all(data)
        .await
        .map_err(LlmError::Write)?;

    Ok(())
}

async fn read_frame(
    stream: &mut UnixStream,
) -> Result<Vec<u8>, LlmError> {
    let mut len_buf: [u8; 4] = [0u8; 4];

    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|error| {
            if error.kind() == io::ErrorKind::UnexpectedEof {
                LlmError::UnexpectedEof
            } else {
                LlmError::Read(error)
            }
        })?;

    let len: usize = u32::from_le_bytes(len_buf) as usize;

    if len > MAX_FRAME_SIZE {
        return Err(
            LlmError::FrameTooLarge { size: len }
        );
    }

    let mut payload: Vec<u8> = vec![0u8; len];

    stream
        .read_exact(&mut payload)
        .await
        .map_err(|error| {
            if error.kind() == io::ErrorKind::UnexpectedEof {
                LlmError::UnexpectedEof
            } else {
                LlmError::Read(error)
            }
        })?;

    Ok(payload)
}

fn parse_end_event(
    payload: &[u8],
) -> Option<GenerationStopReason> {
    let event =
        serde_json::from_slice::<EndEvent>(payload)
            .ok()?;

    if event.event != "end" {
        return None;
    }

    Some(event.reason)
}
