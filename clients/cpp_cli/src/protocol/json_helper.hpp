#ifndef JSON_HELPER_HPP
#define JSON_HELPER_HPP

#include <string>
#include <vector>
#include <map>
#include <sstream>

namespace protocol {

    /**
     * A very minimal JSON helper for the CardArena protocol.
     * Since our protocol is flat and predictable, we use simple string parsing 
     * to avoid heavy dependencies like nlohmann/json in a minimal CLI.
     */
    class JsonHelper {
    public:
        static std::string get_string(const std::string& json, const std::string& key) {
            std::string search_key = "\"" + key + "\":\"";
            size_t start = json.find(search_key);
            if (start == std::string::npos) {
                // Try without quotes for values like numbers or boolean (though we use strings mostly)
                search_key = "\"" + key + "\":";
                start = json.find(search_key);
                if (start == std::string::npos) return "";
                start += search_key.length();
                size_t end = json.find_first_of(",}", start);
                std::string val = json.substr(start, end - start);
                // remove quotes if present
                if (!val.empty() && val.front() == '"') val = val.substr(1, val.length() - 2);
                return val;
            }
            start += search_key.length();
            size_t end = json.find("\"", start);
            return json.substr(start, end - start);
        }

        static int get_int(const std::string& json, const std::string& key) {
            std::string val = get_string(json, key);
            try { return val.empty() ? 0 : std::stoi(val); } catch (...) { return 0; }
        }

        static std::vector<std::string> get_array(const std::string& json, const std::string& key) {
            std::vector<std::string> result;
            std::string search_key = "\"" + key + "\":[";
            size_t start = json.find(search_key);
            if (start == std::string::npos) return result;
            
            start += search_key.length();
            size_t end = json.find("]", start);
            std::string array_content = json.substr(start, end - start);
            
            std::stringstream ss(array_content);
            std::string item;
            while (std::getline(ss, item, ',')) {
                // Clean up quotes and spaces
                size_t q1 = item.find('"');
                size_t q2 = item.find('"', q1 + 1);
                if (q1 != std::string::npos && q2 != std::string::npos) {
                    result.push_back(item.substr(q1 + 1, q2 - q1 - 1));
                }
            }
            return result;
        }

        // Builder
        static std::string build_play(const std::string& card) {
            return "{\"type\":\"PLAY\",\"card\":\"" + card + "\"}";
        }

        static std::string build_hello(const std::string& name) {
            return "{\"type\":\"HELLO\",\"role\":\"HUMAN\",\"nickname\":\"" + name + "\",\"proto\":1}";
        }
    };

} // namespace protocol

#endif // JSON_HELPER_HPP
