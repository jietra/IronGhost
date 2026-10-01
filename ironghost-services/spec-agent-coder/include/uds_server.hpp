#pragma once
#include <string>
#include <functional>
#include "llm.hpp"

class UDSServer {
public:
    //using Handler = std::function<std::string(const std::string&)>;
    using Handler = std::function<void(const std::string&)>;

    UDSServer(LLM& llm, const std::string& socket_path, Handler handler);

    void run();

private:
    std::string socket_path;
    Handler     handler;
    LLM&        llm;

    int         create_socket();
    std::string read_request(int client_fd);
    void        send_response(int client_fd, const std::string& response);
};
