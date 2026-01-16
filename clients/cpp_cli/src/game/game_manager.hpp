#ifndef GAME_MANAGER_HPP
#define GAME_MANAGER_HPP

#include "net/tcp_client.hpp"
#include "net/udp_heartbeat.hpp"
#include "game/state.hpp"
#include "protocol/json_helper.hpp"
#include <iostream>
#include <mutex>

namespace game {

    class GameManager {
    public:
        GameManager(net::TcpClient& client, net::UdpHeartbeat& hb);
        
        void handle_message(const std::string& raw_json);
        void render_ui();
        void process_input(const std::string& input);

    private:
        net::TcpClient& client_;
        net::UdpHeartbeat& hb_;
        GameState state_;
        std::mutex state_mutex_;

        void on_welcome(const std::string& json);
        void on_deal(const std::string& json);
        void on_your_turn(const std::string& json);
        void on_play_broadcast(const std::string& json);
        void on_trick_result(const std::string& json);
        void on_game_over(const std::string& json);
        void on_error(const std::string& json);

        // Helpers
        int get_card_value(const std::string& code);
        std::string trim(const std::string& str);
    };

}

#endif // GAME_MANAGER_HPP
