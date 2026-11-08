#pragma once

#include <cstdint>
#include <string>

inline constexpr std::uint32_t MAX_FRAME_SIZE = 1024 * 1024;

enum class FrameType : std::uint8_t {
    Request = 0x01,
    Content = 0x02,
    End     = 0x03,
    Error   = 0x04,
    Status  = 0x05,
    Cancel  = 0x06,
};

struct Frame {
    FrameType   type;
    std::string payload;
};

bool read_frame(
    int     fd,
    Frame&  frame
);

bool send_frame(
    int                 fd,
    FrameType           type,
    const std::string&  payload
);