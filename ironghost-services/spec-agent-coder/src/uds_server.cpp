#include "uds_server.hpp"

#include <sys/socket.h>
#include <sys/un.h>
#include <sys/select.h>
#include <unistd.h>
#include <fcntl.h>
#include <csignal>
#include <cstring>
#include <iostream>

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
    if (running.exchange(false)) {
        char byte = 1;
        // writing 1 byte immediately wakes up select() in run()
        write(shutdown_pipe[1], &byte, 1);
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

std::string UDSServer::read_request(int client_fd) {
    uint32_t len = 0;
    
    // blocking and exact read of the length of the incoming request
    ssize_t bytes_read = read(client_fd, &len, sizeof(len));
    if (bytes_read != sizeof(len)) {
        return "";  // The client has closed the socket or disconnected properly
    }

    std::string buffer(len, '\0');
    size_t total_read = 0;

    // loop to ensure we read the entire request
    while (total_read < len) {
        ssize_t n = read(client_fd, &buffer[total_read], len - total_read);
        if (n <= 0) return "";  // Error or disconnection
        total_read += n;
    }

    return buffer;
}

void UDSServer::run() {
    // catch interrupts
    std::signal(SIGINT,  UDSServer::signal_handler);
    std::signal(SIGTERM, UDSServer::signal_handler);

    int server_fd = create_socket();
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

            // 1. Accept a client (Non-blocking accept)
            int client_fd = accept(server_fd, nullptr, nullptr);
            if (client_fd < 0) continue;

            // 2. Set callback for this specific client
            llm.set_stream_callback(
                [client_fd](const std::string& piece) {

                    // stream to client
                    uint32_t len = piece.size();
                    write(client_fd, &len, sizeof(len));
                    write(client_fd, piece.c_str(), len);
                }
            );

            // blocking dialogue loop, isolation of each client (no multi-threading)
            while (running) {

                // 3. Read request
                std::string request = read_request(client_fd);

                if (request.empty()) {
                    break;  // client closed socket
                }

                // 4. Handle request
                handler(request);
            }

            close(client_fd);
        }
    }

    std::cout << "\n[UDS] Cleaning up resources..." << std::endl;
    
    // RAII destructors will handle closing file descriptors and unlinking the socket path
    
    //close(server_fd);
    //unlink(socket_path.c_str());
}

// Send response to the client. Only used for non-streaming responses.
void UDSServer::send_response(int client_fd, const std::string& response) {
    uint32_t len = response.size();
    write(client_fd, &len, sizeof(len));
    write(client_fd, response.data(), len);
}