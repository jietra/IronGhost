use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use async_stream::stream;
use futures_util::stream::Stream;

pub struct AgentLLM {
    socket_path: String,
}

impl AgentLLM {
    pub fn new(socket_path: &str) -> Self {
        Self { socket_path: socket_path.to_string() }
    }

    pub fn stream_generate<'a>(&'a self, prompt: &'a str) -> impl Stream<Item = String> + 'a {
        stream! {
            if let Ok(mut stream) = UnixStream::connect(&self.socket_path).await {
                let data = prompt.as_bytes();
                let len_bytes = (data.len() as u32).to_le_bytes();

                if stream.write_all(&len_bytes).await.is_err() { return; }
                if stream.write_all(data).await.is_err() { return; }

                let mut utf8_buffer = Vec::new();

                loop {
                    let mut len_buf = [0u8; 4];
                    if stream.read_exact(&mut len_buf).await.is_err() { break; }
                    let len = u32::from_le_bytes(len_buf) as usize;

                    let mut token_buf = vec![0u8; len];
                    if stream.read_exact(&mut token_buf).await.is_err() { break; }

                    if token_buf == b"\n=== END_OF_STREAM ===" { break; }

                    utf8_buffer.extend_from_slice(&token_buf);

                    if let Ok(s) = std::str::from_utf8(&utf8_buffer) {
                        yield s.to_string(); // <-- Émet le fragment directement dans le stream
                        utf8_buffer.clear();
                    }
                }
            }
        }
    }

    /*
    pub async fn generate(&self, prompt: &str) -> String {

        println!("[AGENTLLM] connecting to socket {}...", self.socket_path);

        // connect to socket
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .expect("Unable to connect");

        println!("[AGENTLLM] connected to socket");

        // --- Send prompt ---
        // send length and data
        let data      = prompt.as_bytes();
        let len_bytes = (data.len() as u32).to_le_bytes();

        stream.write_all(&len_bytes).await.unwrap();

        println!("[AGENTLLM] length sent: {}", data.len());

        stream.write_all(data).await.unwrap();
        
        println!("[AGENTLLM] data sent: {}", prompt);

        // --- Read stream ---
        // read tokens till END_OF_STREAM
        let mut utf8_buffer  = Vec::new();
        let mut final_output = String::new();

        loop {
            // read length of token
            let mut len_buf = [0u8; 4];
            if stream.read_exact(&mut len_buf).await.is_err() {

                println!("[AGENTLLM] stream len not exact! break!");

                break;
            }
            let len = u32::from_le_bytes(len_buf) as usize;

            // read token
            let mut token_buf = vec![0u8; len];
            stream.read_exact(&mut token_buf).await.unwrap();

            // end of stream
            if token_buf == b"\n=== END_OF_STREAM ===" {

                println!("\n\n[AGENTLLM] end of stream! break!");

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
    */
}
