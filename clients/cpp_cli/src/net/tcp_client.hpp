#ifndef TCP_CLIENT_HPP
#define TCP_CLIENT_HPP

#include "socket_wrapper.hpp"
#include <string>
#include <atomic>
#include <thread>
#include <functional>
#include <vector>

namespace net {

    class TcpClient {
    public:
        using OnMessageCallback = std::function<void(const std::string&)>;

        TcpClient();
        ~TcpClient();

        bool connect_to(const std::string& host, int port);
        void disconnect();
        bool send_message(const std::string& msg);
        
        void set_on_message(OnMessageCallback cb) {
            on_message_ = cb;
        }

        bool is_connected() const { return connected_; }

    private:
        void receive_loop();

        socket_t sock_;
        std::atomic<bool> connected_;
        std::thread receiver_thread_;
        OnMessageCallback on_message_;
    };

}

#endif // TCP_CLIENT_HPP
