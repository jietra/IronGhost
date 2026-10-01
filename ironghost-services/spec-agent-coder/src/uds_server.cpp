#include "uds_server.hpp"

#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <cstring>
#include <iostream>
#include <sys/socket.h>
#include <csignal>

UDSServer::UDSServer(LLM& llm, const std::string& socket_path, Handler handler)
    : socket_path(socket_path), handler(handler), llm(llm) {}

int UDSServer::create_socket() {

    // create socket
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);   // file descriptor
    if (fd < 0) {
        std::cerr << "ERROR: cannot create UDS socket" << std::endl;
        std::exit(1);
    }

    // prepare address
    sockaddr_un addr{};
    addr.sun_family = AF_UNIX;
    std::strncpy(addr.sun_path, socket_path.c_str(), sizeof(addr.sun_path) - 1);

    // clean previous socket
    unlink(socket_path.c_str());

    // binder
    if (bind(fd, (sockaddr*)&addr, sizeof(addr)) < 0) {
        std::cerr << "ERROR: cannot bind UDS socket: " << socket_path << std::endl;
        std::exit(1);
    }

    // listen
    if (listen(fd, 8) < 0) {
        std::cerr << "ERROR: cannot listen on UDS socket" << std::endl;
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

static volatile std::sig_atomic_t g_running = 1;

void UDSServer::run() {
    std::signal(SIGINT, [](int) { g_running = 0; });    // catch Ctrl+C interrupt

    int server_fd = create_socket();
    std::cout << "LLM server listening on UDS " << socket_path << std::endl;

    while (g_running) {
        // 1. Accept a client
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

        // Dialogue loop
        while(true) {
            // 3. Read request
            std::string request = read_request(client_fd);
            if (request.empty()) {
                break; // client closed the socket
            }
            // 4. Handle request
            //std::string response = handler(request);
            handler(request);
            
            // 5. No need to send response: response already streamed
            //send_response(client_fd, response);
        }
        close(client_fd);
    }

    close(server_fd);
    unlink(socket_path.c_str());    // proper cleaning
}
