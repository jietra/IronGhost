#pragma once

#include <string>
#include <functional>
#include <atomic>
#include "llm.hpp"

class UDSServer {
public:
    using Handler = std::function<void(const std::string&)>;

    UDSServer(LLM& llm, const std::string& socket_path, Handler handler);
    ~UDSServer();   // Destructor to clean up resources

    void run();
    void stop();    // Method to stop the server gracefully

private:
    std::string socket_path;
    Handler     handler;
    LLM&        llm;

    int                 server_fd{-1};      // File descriptor for the server socket
    int                 shutdown_pipe[2]{-1, -1};       // Pipe for signaling shutdown
    std::atomic<bool>   running{false};     // Atomic flag to indicate if the server is running

    static UDSServer* instance;         // Static instance pointer for signal handling

    /// @brief Signal handler for graceful shutdown
    /// @param sig Signal number
    static void signal_handler(int sig);

    int         create_socket();
    std::string read_request(int client_fd);

    /// @brief Send response to the client. Only used for non-streaming responses.
    /// @param client_fd File descriptor of the client socket.
    /// @param response Response string to send.
    void        send_response(int client_fd, const std::string& response);
};
