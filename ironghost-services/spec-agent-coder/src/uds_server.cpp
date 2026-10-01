#include "uds_server.hpp"

#include <sys/socket.h>
#include <sys/un.h>
#include <sys/select.h>
#include <fcntl.h>
#include <unistd.h>
#include <cstring>
#include <iostream>
#include <sys/socket.h>
#include <csignal>
#include <atomic>
#include <cerrno>

UDSServer::UDSServer(LLM& llm, const std::string& socket_path, Handler handler)
    : socket_path(socket_path), handler(handler), llm(llm) {}

int UDSServer::create_socket() {
    // clean previous socket
    unlink(socket_path.c_str());

    // create socket
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);   // file descriptor
    if (fd < 0) {
        std::cerr << "[UDS] ERROR: cannot create UDS socket" << std::endl;
        std::exit(1);
    }

    // Pass socket to NON-BLOCKING mode so accept() never hangs
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags >= 0) {
        fcntl(fd, F_SETFL, flags | O_NONBLOCK);
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
    if (read(client_fd, &len, sizeof(len)) != sizeof(len)) {
        return "";
    }

    std::string buffer(len, '\0');
    if (read(client_fd, &buffer[0], len) != (ssize_t)len) {
        return "";
    }

    return buffer;
}

void UDSServer::send_response(int client_fd, const std::string& response) {
    uint32_t len = response.size();
    write(client_fd, &len, sizeof(len));
    write(client_fd, response.data(), len);
}

static std::atomic<bool> g_running{true};

static void handle_sigint(int) {
    g_running = false;
}

void UDSServer::run() {
    // catch Ctrl+C interrupt
    std::signal(SIGINT, handle_sigint);
    std::signal(SIGTERM, handle_sigint);

    int server_fd = create_socket();
    std::cout << "[UDS] LLM server listening on UDS " << socket_path << std::endl;

    while (g_running) {
        // --- Use select() with 500ms timeout to periodically check g_running ---
        fd_set readfds;
        FD_ZERO(&readfds);
        FD_SET(server_fd, &readfds);

        struct timeval tv;
        tv.tv_sec  = 0;
        tv.tv_usec = 500000; // 500 ms check interval

        int activity = select(server_fd + 1, &readfds, nullptr, nullptr, &tv);

        if (activity <= 0) {
            continue;
        }

        // 1. Accept a client (Non-blocking accept)
        int client_fd = accept(server_fd, nullptr, nullptr);
        if (client_fd < 0) {
            continue;
        }

        // 2. Set callback for this specific client
        llm.set_stream_callback(
            [client_fd](const std::string& piece) {
                // stream to client
                uint32_t len = piece.size();
                write(client_fd, &len, sizeof(len));
                write(client_fd, piece.c_str(), len);
            }
        );

        // Dialogue loop
        while(g_running) {
            // 3. Read request
            std::string request = read_request(client_fd);

            if (request.empty()) {
                break; // client closed socket
            }
            // 4. Handle request
            handler(request);
        }
        close(client_fd);
    }

    close(server_fd);
    unlink(socket_path.c_str());    // proper cleaning
    std::cout << "\n[UDS] clean shutdown complete." << std::endl;
}
