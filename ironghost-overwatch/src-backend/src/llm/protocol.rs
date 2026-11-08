// src/llm/protocol.rs

use tokio::io::{
    AsyncRead,
    AsyncReadExt,
    AsyncWrite,
    AsyncWriteExt,
};

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("unknown frame type: 0x{0:02x}")]
    UnknownFrameType(u8),

    #[error("frame too large: {0} bytes")]
    FrameTooLarge(usize),

    #[error("protocol I/O error: {0}")]
    Io(
        #[from]
        std::io::Error
    ),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Request = 0x01,
    Content = 0x02,
    End     = 0x03,
    Error   = 0x04,
    Status  = 0x05,
    Cancel  = 0x06,
}

#[derive(Debug)]
pub struct Frame {
    pub kind    : FrameType,
    pub payload : Vec<u8>,
}

/*
 * TODO: move to configuration if the limit
 * needs to vary by deployment.
 */
const MAX_FRAME_SIZE: usize = 1024 * 1024;

impl TryFrom<u8> for FrameType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0x01 => Ok(Self::Request),
            0x02 => Ok(Self::Content),
            0x03 => Ok(Self::End),
            0x04 => Ok(Self::Error),
            0x05 => Ok(Self::Status),
            0x06 => Ok(Self::Cancel),

            value => Err(
                ProtocolError::UnknownFrameType(value)
            ),
        }
    }
}

pub async fn read_frame<R>(stream: &mut R) -> Result<Frame, ProtocolError> where R: AsyncRead + Unpin {
    
    let mut header: [u8; 5] = [0u8; 5];

    stream.read_exact(&mut header).await?;

    let kind: FrameType = FrameType::try_from(header[0])?;

    let length: usize = u32::from_be_bytes([
        header[1],
        header[2],
        header[3],
        header[4],
    ]) as usize;

    if length > MAX_FRAME_SIZE {
        return Err(
            ProtocolError::FrameTooLarge(
                length
            )
        );
    }

    let mut payload: Vec<u8> = vec![0u8; length];

    if length > 0 {
        stream.read_exact(&mut payload).await?;
    }

    Ok(Frame {
        kind,
        payload,
    })
}

pub async fn write_frame<W>(
    stream  : &mut W,
    kind    : FrameType,
    payload : &[u8],
) -> Result<(), ProtocolError> where W: AsyncWrite + Unpin {

    if payload.len() > MAX_FRAME_SIZE {
        return Err(
            ProtocolError::FrameTooLarge(
                payload.len()
            )
        );
    }

    let length: u32 = u32::try_from(payload.len())
        .map_err(|_| {
            ProtocolError::FrameTooLarge(
                payload.len()
            )
        })?;

    let mut header: [u8; 5] = [0u8; 5];
    
    header[0] = kind as u8;

    header[1..5].copy_from_slice(&length.to_be_bytes());

    stream.write_all(&header).await?;

    if !payload.is_empty() {
        stream.write_all(payload).await?;
    }

    Ok(())
}