#include "uds_protocol.hpp"

#include <sys/socket.h>
#include <arpa/inet.h>
#include <cerrno>
#include <cstddef>
#include <cstdint>
#include <utility>

namespace {

    bool read_all(
        int    fd,
        void*  data,
        size_t size
    ) {
        auto* bytes = static_cast<std::uint8_t*>(data);

        size_t received = 0;

        while (received < size) {
            const ssize_t n = recv(
                fd,
                bytes + received,
                size - received,
                0
            );

            if (n > 0) {
                received += static_cast<size_t>(n);
                continue;
            }

            if (n < 0 && errno == EINTR) {
                continue;
            }

            return false;
        }

        return true;
    }

    bool write_all(
        int         fd,
        const void* data,
        size_t      size
    ) {
        const auto* bytes = static_cast<const std::uint8_t*>(data);

        size_t written = 0;

        while (written < size) {
            const ssize_t n = send(
                fd,
                bytes + written,
                size - written,
                MSG_NOSIGNAL
            );

            if (n > 0) {
                written += static_cast<size_t>(n);
                continue;
            }

            if (n < 0 && errno == EINTR) {
                continue;
            }

            return false;
        }

        return true;
    }

} // namespace

bool send_frame(
    int                 fd,
    FrameType           type,
    const std::string&  payload
) {
    if (payload.size() > MAX_FRAME_SIZE) {
        return false;
    }

    const std::uint32_t length      = static_cast<std::uint32_t>(payload.size());
    const std::uint32_t length_be   = htonl(length);
    const std::uint8_t  raw_type    = static_cast<std::uint8_t>(type);

    return
        write_all(fd, &raw_type, sizeof(raw_type))      &&
        write_all(fd, &length_be, sizeof(length_be))    &&
        write_all(fd, payload.data(), payload.size());
}

bool read_frame(
    int fd,
    Frame& frame
) {
    std::uint8_t    raw_type    = 0;
    std::uint32_t   length_be   = 0;

    if (!read_all(fd, &raw_type, sizeof(raw_type))) {
        return false;
    }

    if (!read_all(fd, &length_be, sizeof(length_be))) {
        return false;
    }

    const std::uint32_t length = ntohl(length_be);

    if (length > MAX_FRAME_SIZE) {
        return false;
    }

    std::string payload(static_cast<std::size_t>(length),'\0');

    if (
        length > 0 &&
        !read_all(fd, payload.data(), payload.size())
    ) {
        return false;
    }

    switch (static_cast<FrameType>(raw_type)) {
        case FrameType::Request:
        case FrameType::Content:
        case FrameType::End:
        case FrameType::Error:
        case FrameType::Status:
        case FrameType::Cancel:
            break;

        default:
            return false;
    }

    frame.type      = static_cast<FrameType>(raw_type);
    frame.payload   = std::move(payload);

    return true;
}