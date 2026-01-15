#include <iostream>
#include <string>
#include <thread>
#include <chrono>
#include "net/tcp_client.hpp"

// Simple main to test S8.1 & S8.2
int main() {
    std::cout << "=== CardArena C++ Client (CLI) ===" << std::endl;
    std::cout << "Connecting to 127.0.0.1:8888..." << std::endl;

    net::TcpClient client;

    // Set callback for incoming messages
    client.set_on_message([](const std::string& msg) {
        std::cout << "[RX] " << msg << std::endl;
    });

    if (client.connect_to("127.0.0.1", 8888)) {
        std::cout << "Connected! Type JSON messages to send (or 'quit' to exit)." << std::endl;
        
        // Manual handshake for testing
        // {"type":"HELLO","role":"HUMAN","nickname":"CppUser","proto":1}
        std::string hello = "{\"type\":\"HELLO\",\"role\":\"HUMAN\",\"nickname\":\"CppUser\",\"proto\":1}";
        client.send_message(hello);

        std::string input;
        while (client.is_connected()) {
            std::getline(std::cin, input);
            if (input == "quit") break;
            if (!input.empty()) {
                client.send_message(input);
            }
        }
    } else {
        std::cerr << "Failed to connect." << std::endl;
    }

    client.disconnect();
    return 0;
}
