#include "uds_server.hpp"
#include "uds_protocol.hpp"

#include <sys/socket.h>
#include <sys/un.h>
#include <sys/select.h>
#include <unistd.h>
#include <fcntl.h>
#include <csignal>
#include <cstring>
#include <iostream>
#include <cerrno>

UDSServer* UDSServer::instance = nullptr;

UDSServer::UDSServer(LLM& llm, const std::string& socket_path, Handler handler)
    : socket_path(socket_path), handler(handler), llm(llm) {
    
    // creation of the self-interrupt pipe
    if (pipe(shutdown_pipe) < 0) {
        perror("[UDS] pipe creation failed");
        std::exit(1);
    }

    // write end of the pipe must be non-blocking to avoid any blocking in the signal handler
    fcntl(shutdown_pipe[1], F_SETFL, O_NONBLOCK);

    instance = this;
}

UDSServer::~UDSServer() {
    if (server_fd >= 0) close(server_fd);
    if (shutdown_pipe[0] >= 0) close(shutdown_pipe[0]);
    if (shutdown_pipe[1] >= 0) close(shutdown_pipe[1]);
    unlink(socket_path.c_str());
}

void UDSServer::signal_handler(int) {
    if (instance) {
        instance->stop();
    }
}

void UDSServer::stop() {
    llm.request_cancel();

    if (running.exchange(false)) {
        char byte = 1;
        
        // writing 1 byte immediately wakes up select() in run()
        const ssize_t ignored = write(shutdown_pipe[1], &byte, 1);

        (void)ignored;
    }
}

int UDSServer::create_socket() {
    // clean previous socket
    unlink(socket_path.c_str());

    // create socket
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);   // file descriptor
    if (fd < 0) {
        std::cerr << "[UDS] ERROR: cannot create UDS socket" << std::endl;
        std::exit(1);
    }

    // prepare address
    sockaddr_un addr{};
    addr.sun_family = AF_UNIX;
    std::strncpy(addr.sun_path, socket_path.c_str(), sizeof(addr.sun_path) - 1);

    // binder
    if (bind(fd, (sockaddr*)&addr, sizeof(addr)) < 0) {
        std::cerr << "[UDS] ERROR: cannot bind UDS socket: " << socket_path << std::endl;
        std::exit(1);
    }

    // listen
    if (listen(fd, 8) < 0) {
        std::cerr << "[UDS] ERROR: cannot listen on UDS socket" << std::endl;
        std::exit(1);
    }

    return fd;
}

void UDSServer::run() {
    // catch interrupts
    std::signal(SIGINT,  UDSServer::signal_handler);
    std::signal(SIGTERM, UDSServer::signal_handler);

    server_fd = create_socket();
    running = true;

    std::cout << "[UDS] LLM server listening on UDS " << socket_path << std::endl;

    while (running) {
        fd_set readfds;
        FD_ZERO(&readfds);
        FD_SET(server_fd, &readfds);
        FD_SET(shutdown_pipe[0], &readfds);

        int max_fd = std::max(server_fd, shutdown_pipe[0]);

        // blocking wait without timeout (no CPU waste)
        // unlocked only if a client connects or if stop() is called (via signal)
        int activity = select(max_fd + 1, &readfds, nullptr, nullptr, nullptr);

        if (activity < 0) {
            if (errno == EINTR) continue;   // interrupted by signal, continue
            break;  // other error, exit loop
        }

        // if the activity comes from the shutdown_pipe, we stop gracefully
        if (FD_ISSET(shutdown_pipe[0], &readfds)) {
            break;
        }

        // if the activity comes from the server_fd, we accept a client
        if (FD_ISSET(server_fd, &readfds)) {

            // 1. Accept ready client connection
            const int client_fd = accept(server_fd, nullptr, nullptr);
            
            if (client_fd < 0) continue;

            std::cout
                << "[UDS] accepted client fd="
                << client_fd
                << '\n';

            // 2. Set callback for this specific client
            llm.set_stream_callback(
                [this, client_fd](const std::string& piece) {

                    // stream to client
                    if (!send_frame(
                        client_fd,
                        FrameType::Content,
                        piece
                    )) {
                        std::cerr
                            << "[UDS] Client disconnected; "
                            << "cancelling generation\n";

                        llm.request_cancel();
                    }
                }
            );

            // 3. Read request
            Frame frame;

            if (!read_frame(client_fd, frame)) {
                llm.set_stream_callback(nullptr);
                close(client_fd);
                continue;
            }

            if (frame.type != FrameType::Request) {
                send_frame(
                    client_fd,
                    FrameType::Error,
                    R"({"code":"invalid_frame","message":"expected REQUEST"})"
                );
                llm.set_stream_callback(nullptr);
                close(client_fd);
                continue;
            }

            // 4. Handle request
            const GenerationResult result = handler(frame.payload);

            const FrameType terminal_type =
                is_generation_error(result.reason)
                    ? FrameType::Error
                    : FrameType::End;

            const std::string terminal_payload =
                std::string(R"({"reason":")")
                + stop_reason_to_string(result.reason)
                + R"("})";

            if (!send_frame(
                client_fd,
                terminal_type,
                terminal_payload
            )) {
                std::cerr
                    << "[UDS] failed to send END frame\n";
            }

            std::cout
                << "[UDS] generation end fd="
                << client_fd
                << '\n';

            llm.set_stream_callback(nullptr);

            close(client_fd);

            std::cout
                << "[UDS] closed client fd="
                << client_fd
                << '\n';

        }
    }

    std::cout << "\n[UDS] Cleaning up resources..." << std::endl;
    
    // RAII destructors will handle closing file descriptors and unlinking the socket path
}

// Send response to the client. Only used for non-streaming responses.
void UDSServer::send_response(int client_fd, const std::string& response) {

    if (!send_frame(
        client_fd,
        FrameType::Content,
        response
    )) {
        std::cerr
            << "[UDS] failed to send response\n";
    }
}