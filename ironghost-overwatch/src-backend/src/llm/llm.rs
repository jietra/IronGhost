// src/llm/llm.rs

use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use async_stream::stream;
use futures_util::stream::Stream;

pub struct LLM {
    socket_path: String,
}

impl LLM {
    pub fn new(socket_path: &str) -> Self {
        Self { socket_path: socket_path.to_string() }
    }

    pub fn stream_generate<'a>(&'a self, prompt: &'a str) -> impl Stream<Item = String> + 'a {
        stream! {
            println!("[LLM] connecting to socket {}...", self.socket_path);

            // connect to socket
            if let Ok(mut stream) = UnixStream::connect(&self.socket_path).await {
                println!("[LLM] connected to socket");

                // --- Send prompt ---
                // send length and data
                let data      = prompt.as_bytes();
                let len_bytes = (data.len() as u32).to_le_bytes();

                if stream.write_all( &len_bytes ).await.is_err() { return; }
                if stream.write_all( data       ).await.is_err() { return; }

                // --- Read stream ---
                // read tokens till END_OF_STREAM
                let mut utf8_buffer = Vec::new();

                loop {
                    // read length of token
                    let mut len_buf = [0u8; 4];
                    if stream.read_exact( &mut len_buf   ).await.is_err() {
                        println!("[LLM] stream len not exact! break!");
                        break;
                    }
                    let len = u32::from_le_bytes(len_buf) as usize;

                    // read token
                    let mut token_buf = vec![0u8; len];
                    if stream.read_exact( &mut token_buf ).await.is_err() { break; }

                    // end of stream
                    if token_buf == b"\n=== END_OF_STREAM ===" {
                        println!("\n\n[LLM] end of stream! break!");
                        break;
                    }

                    // accumulate UTF-8 fragments
                    utf8_buffer.extend_from_slice(&token_buf);

                    if let Ok(s) = std::str::from_utf8(&utf8_buffer) {
                        yield s.to_string();    // <-- Emit chunk directly into stream
                        utf8_buffer.clear();
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
