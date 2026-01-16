#include <iostream>
#include <string>
#include "net/tcp_client.hpp"
#include "game/game_manager.hpp"

int main(int argc, char* argv[]) {
    std::cout << "=== CardArena C++ Client (CLI) ===" << std::endl;
    
    std::string host = "127.0.0.1";
    int port = 8888;

    if (argc > 1) {
        host = argv[1];
    }
    
    std::cout << "Target Server: " << host << ":" << port << std::endl;

    std::string nickname;

    std::cout << "Enter Nickname: ";
    std::getline(std::cin, nickname);
    if (nickname.empty()) nickname = "CppPlayer";

    net::TcpClient client;
    net::UdpHeartbeat hb;
    game::GameManager manager(client, hb);

    client.set_on_message([&](const std::string& msg) {
        manager.handle_message(msg);
    });

    if (client.connect_to(host, port)) {
        hb.start(host, port + 1); // UDP port is TCP port + 1
        client.send_message(protocol::JsonHelper::build_hello(nickname));

        std::string input;
        while (client.is_connected()) {
            if (std::getline(std::cin, input)) {
                if (input == "quit" || input == "exit") break;
                manager.process_input(input);
            }
        }
        hb.stop();
    } else {
        std::cerr << "Failed to connect to " << host << ":" << port << std::endl;
    }

    client.disconnect();
    return 0;
}
