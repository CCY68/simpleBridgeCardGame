#include "udp_heartbeat.hpp"
#include <iostream>
#include <vector>
#include <cstring>
#include <sstream>

namespace net {

    UdpHeartbeat::UdpHeartbeat() 
        : sock_(INVALID_SOCKET), port_(0), running_(false), 
          seq_counter_(0), received_count_(0), last_rtt_(0.0), loss_rate_(0.0) {
        net::initialize();
    }

    UdpHeartbeat::~UdpHeartbeat() {
        stop();
        net::cleanup();
    }

    bool UdpHeartbeat::start(const std::string& host, int port) {
        host_ = host;
        port_ = port;

        sock_ = socket(AF_INET, SOCK_DGRAM, 0);
        if (!IS_VALID_SOCKET(sock_)) {
            return false;
        }

        running_ = true;
        send_thread_ = std::thread(&UdpHeartbeat::send_loop, this);
        recv_thread_ = std::thread(&UdpHeartbeat::recv_loop, this);
        
        return true;
    }

    void UdpHeartbeat::stop() {
        running_ = false;
        if (IS_VALID_SOCKET(sock_)) {
            CLOSE_SOCKET(sock_);
            sock_ = INVALID_SOCKET;
        }
        if (send_thread_.joinable()) send_thread_.join();
        if (recv_thread_.joinable()) recv_thread_.join();
    }

    void UdpHeartbeat::send_loop() {
        struct sockaddr_in serv_addr;
        std::memset(&serv_addr, 0, sizeof(serv_addr));
        serv_addr.sin_family = AF_INET;
        serv_addr.sin_port = htons(port_);
        serv_addr.sin_addr.s_addr = inet_addr(host_.c_str());

        while (running_) {
            uint32_t seq = ++seq_counter_;
            uint64_t now = get_time_ms();

            std::string msg = "{\"type\":\"HB_PING\",\"seq\":";
            msg += std::to_string(seq);
            msg += ",\"t_client_ms\":";
            msg += std::to_string(now);
            msg += "}\n";

            sendto(sock_, msg.c_str(), (int)msg.length(), 0, (struct sockaddr*)&serv_addr, sizeof(serv_addr));

            if (seq > 0) {
                loss_rate_ = 1.0 - ((double)received_count_ / (double)seq);
            }

            std::this_thread::sleep_for(std::chrono::seconds(1));
        }
    }

    void UdpHeartbeat::recv_loop() {
        char buffer[1024];
        while (running_) {
            struct sockaddr_in from;
            socklen_t from_len = sizeof(from);
            int len = recvfrom(sock_, buffer, sizeof(buffer) - 1, 0, (struct sockaddr*)&from, &from_len);
            
            if (len > 0) {
                buffer[len] = '\0';
                std::string msg(buffer);
                
                size_t t_pos = msg.find("\"t_client_ms\":");
                if (t_pos != std::string::npos) {
                    size_t start = t_pos + 14;
                    size_t end = msg.find_first_of(",}", start);
                    try {
                        std::string t_str = msg.substr(start, end - start);
                        if (t_str.length() > 0 && t_str.front() == '"') t_str = t_str.substr(1, t_str.length() - 2);
                        uint64_t t_sent = std::stoull(t_str);
                        last_rtt_ = (double)(get_time_ms() - t_sent);
                        received_count_++;
                    } catch (...) {}
                }
            } else {
                if (running_) std::this_thread::sleep_for(std::chrono::milliseconds(100));
            }
        }
    }

}
