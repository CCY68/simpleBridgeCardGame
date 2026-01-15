#ifndef GAME_STATE_HPP
#define GAME_STATE_HPP

#include <string>
#include <vector>
#include <map>

namespace game {

    struct Card {
        std::string code; // e.g., "AS", "10H"
        
        std::string to_string() const { return code; }
    };

    struct TablePlay {
        std::string player_id;
        std::string card;
    };

    struct Score {
        int human = 0;
        int ai = 0;
    };

    class GameState {
    public:
        std::string player_id;
        std::string nickname;
        std::string room;
        
        std::vector<Card> hand;
        std::vector<TablePlay> table;
        std::vector<Card> legal_moves;
        
        Score score;
        int current_trick = 0;
        int total_tricks = 0;
        
        bool my_turn = false;

        void reset_table() {
            table.clear();
        }
    };

}

#endif // GAME_STATE_HPP
