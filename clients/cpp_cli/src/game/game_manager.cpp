#include "game_manager.hpp"
#include <algorithm>

namespace game {

    GameManager::GameManager(net::TcpClient& client, net::UdpHeartbeat& hb) 
        : client_(client), hb_(hb) {}

    void GameManager::handle_message(const std::string& json) {
        std::lock_guard<std::mutex> lock(state_mutex_);
        std::string type = protocol::JsonHelper::get_string(json, "type");

        if (type == "WELCOME") on_welcome(json);
        else if (type == "DEAL") on_deal(json);
        else if (type == "YOUR_TURN") on_your_turn(json);
        else if (type == "PLAY_BROADCAST") on_play_broadcast(json);
        else if (type == "TRICK_RESULT") on_trick_result(json);
        else if (type == "GAME_OVER") on_game_over(json);
        else if (type == "ERROR") on_error(json);
        
        render_ui();
    }

    void GameManager::on_welcome(const std::string& json) {
        state_.player_id = protocol::JsonHelper::get_string(json, "player_id");
        state_.nickname = protocol::JsonHelper::get_string(json, "nickname");
        state_.room = protocol::JsonHelper::get_string(json, "room");
        std::cout << "\n>>> Welcome! You are " << state_.nickname << " (" << state_.player_id << ") in room " << state_.room << std::endl;
    }

    void GameManager::on_deal(const std::string& json) {
        state_.hand.clear();
        auto cards = protocol::JsonHelper::get_array(json, "hand");
        for (const auto& c : cards) state_.hand.push_back({c});
        state_.total_tricks = protocol::JsonHelper::get_int(json, "total_tricks");
        state_.current_trick = 1;
        state_.score = {0, 0};
        state_.reset_table();
        std::cout << "\n>>> Cards Dealt! Game Started." << std::endl;
    }

    void GameManager::on_your_turn(const std::string& json) {
        state_.my_turn = true;
        state_.current_trick = protocol::JsonHelper::get_int(json, "trick");
        state_.legal_moves.clear();
        auto legals = protocol::JsonHelper::get_array(json, "legal");
        for (const auto& c : legals) state_.legal_moves.push_back({c});
        
        std::cout << "\n*** YOUR TURN! ***" << std::endl;
    }

    void GameManager::on_play_broadcast(const std::string& json) {
        std::string pid = protocol::JsonHelper::get_string(json, "player_id");
        std::string card = protocol::JsonHelper::get_string(json, "card");
        state_.table.push_back({pid, card});
        
        if (pid == state_.player_id) {
            state_.my_turn = false;
            // Remove from hand
            state_.hand.erase(std::remove_if(state_.hand.begin(), state_.hand.end(), 
                [&](const Card& c){ return c.code == card; }), state_.hand.end());
        }
    }

    void GameManager::on_trick_result(const std::string& json) {
        std::string winner = protocol::JsonHelper::get_string(json, "winner");
        state_.score.human = protocol::JsonHelper::get_int(json, "human_score"); // Simplified score parsing
        state_.score.ai = protocol::JsonHelper::get_int(json, "ai_score");
        
        std::cout << "\n>>> Trick Result: Winner is " << winner << std::endl;
        state_.reset_table();
    }

    void GameManager::on_game_over(const std::string& json) {
        std::string winner = protocol::JsonHelper::get_string(json, "winner");
        std::cout << "\n===============================" << std::endl;
        std::cout << "   GAME OVER! Winner: " << winner << std::endl;
        std::cout << "===============================" << std::endl;
    }

    void GameManager::on_error(const std::string& json) {
        std::string msg = protocol::JsonHelper::get_string(json, "message");
        std::cerr << "\n[!] Server Error: " << msg << std::endl;
    }

    void GameManager::render_ui() {
        // Simple CLI view
        // Using a buffer to prevent output interleaving
        std::stringstream ss;
        ss << "\n------------------------------------------" << std::endl;
        ss << " Trick: " << state_.current_trick << "/" << state_.total_tricks 
           << " | Score: H:" << state_.score.human << " A:" << state_.score.ai << std::endl;
        
        ss << " Net: RTT=" << hb_.get_last_rtt() << "ms Loss=" << (hb_.get_loss_rate() * 100.0) << "%" << std::endl;

        ss << " Table: ";
        if (state_.table.empty()) ss << "(empty)";
        for (const auto& p : state_.table) ss << "[" << p.player_id << ":" << p.card << "] ";
        ss << std::endl;

        ss << " Hand: ";
        for (size_t i = 0; i < state_.hand.size(); ++i) {
            ss << "(" << i << ")" << state_.hand[i].code << " ";
        }
        ss << std::endl;

        if (state_.my_turn) {
            ss << " Legal Moves: ";
            for (const auto& c : state_.legal_moves) ss << c.code << " ";
            ss << "\n Enter card to play (e.g. 'AS', '10H') or 'auto': " << std::flush;
        } else {
            ss << " Waiting for other players..." << std::endl;
        }
        
        std::cout << ss.str();
    }

    void GameManager::process_input(const std::string& raw_input) {
        std::lock_guard<std::mutex> lock(state_mutex_);
        if (!state_.my_turn) return;

        std::string input = trim(raw_input);
        if (input.empty()) return;

        // Convert input to uppercase for case-insensitive matching
        std::string card_code = input;
        std::transform(card_code.begin(), card_code.end(), card_code.begin(), ::toupper);

        try {
            // Strategy: Auto Play (Max Rank)
            if (card_code == "AUTO" || card_code == "A") {
                if (state_.legal_moves.empty()) return;
                
                // Find the best card (highest rank) in legal moves
                std::string best_card = state_.legal_moves[0].code;
                int max_val = -1;

                for (const auto& c : state_.legal_moves) {
                    int val = get_card_value(c.code);
                    if (val > max_val) {
                        max_val = val;
                        best_card = c.code;
                    }
                }
                
                std::cout << ">> Auto-playing highest rank: " << best_card << std::endl;
                client_.send_message(protocol::JsonHelper::build_play(best_card));
                return;
            }

            // Manual Play by Card Code
            // Check if user input matches a card in hand
            bool in_hand = false;
            for (const auto& c : state_.hand) {
                if (c.code == card_code) {
                    in_hand = true;
                    break;
                }
            }

            if (!in_hand) {
                std::cout << ">> You don't have card '" << card_code << "'." << std::endl;
                return;
            }

            // Check legality
            bool is_legal = false;
            for (const auto& c : state_.legal_moves) {
                if (c.code == card_code) { is_legal = true; break; }
            }

            if (is_legal) {
                client_.send_message(protocol::JsonHelper::build_play(card_code));
            } else {
                std::cout << ">> Illegal move! Please select from legal moves." << std::endl;
            }

        } catch (...) {
            std::cout << ">> Error processing input." << std::endl;
        }
    }

    int GameManager::get_card_value(const std::string& code) {
        if (code.length() < 2) return 0;
        // Last char is suit, rest is rank
        std::string rank_str = code.substr(0, code.length() - 1);
        
        if (rank_str == "A") return 14;
        if (rank_str == "K") return 13;
        if (rank_str == "Q") return 12;
        if (rank_str == "J") return 11;
        try {
            return std::stoi(rank_str);
        } catch (...) {
            return 0;
        }
    }

    std::string GameManager::trim(const std::string& str) {
        size_t first = str.find_first_not_of(" \t\r\n");
        if (std::string::npos == first) return str;
        size_t last = str.find_last_not_of(" \t\r\n");
        return str.substr(first, (last - first + 1));
    }

} // namespace game
