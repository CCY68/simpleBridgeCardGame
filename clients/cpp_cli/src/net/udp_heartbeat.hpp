#ifndef UDP_HEARTBEAT_HPP
#define UDP_HEARTBEAT_HPP

#include "socket_wrapper.hpp"
#include <string>
#include <atomic>
#include <thread>

namespace net {

    class UdpHeartbeat {
    public:
        UdpHeartbeat();
        ~UdpHeartbeat();

        bool start(const std::string& host, int port);
        void stop();

        // Stats
        double get_last_rtt() const { return last_rtt_; }
        double get_loss_rate() const { return loss_rate_; }

    private:
        void send_loop();
        void recv_loop();

        socket_t sock_;
        std::string host_;
        int port_;
        
        std::atomic<bool> running_;
        std::thread send_thread_;
        std::thread recv_thread_;

        // Metrics
        std::atomic<uint32_t> seq_counter_;
        std::atomic<uint32_t> received_count_;
        std::atomic<double> last_rtt_;
        std::atomic<double> loss_rate_;

        struct PingInfo {
            uint32_t seq;
            uint64_t sent_at;
        };
    };

}

#endif // UDP_HEARTBEAT_HPP
