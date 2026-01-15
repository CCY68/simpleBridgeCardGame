#include "tcp_client.hpp"
#include <iostream>
#include <vector>
#include <cstring>

namespace net {

    TcpClient::TcpClient() : sock_(INVALID_SOCKET), connected_(false) {
        net::initialize();
    }

    TcpClient::~TcpClient() {
        disconnect();
        net::cleanup();
    }

    bool TcpClient::connect_to(const std::string& host, int port) {
        if (connected_) return true;

        // Create socket
        sock_ = socket(AF_INET, SOCK_STREAM, 0);
        if (!IS_VALID_SOCKET(sock_)) {
            std::cerr << "Error creating socket: " << SOCKET_ERROR_CODE << std::endl;
            return false;
        }

        // Resolve address
        struct sockaddr_in server_addr;
        server_addr.sin_family = AF_INET;
        server_addr.sin_port = htons(port);
        
        #ifdef _WIN32
            // Windows specific inet_pton might need Ws2_32.lib or newer Windows
            // For older compatibility or simplicity:
            server_addr.sin_addr.s_addr = inet_addr(host.c_str());
        #else
            if (inet_pton(AF_INET, host.c_str(), &server_addr.sin_addr) <= 0) {
                 std::cerr << "Invalid address/ Address not supported" << std::endl;
                 return false;
            }
        #endif

        // Connect
        if (connect(sock_, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
            std::cerr << "Connection failed: " << SOCKET_ERROR_CODE << std::endl;
            CLOSE_SOCKET(sock_);
            return false;
        }

        connected_ = true;
        
        // Start receiver thread
        receiver_thread_ = std::thread(&TcpClient::receive_loop, this);

        return true;
    }

    void TcpClient::disconnect() {
        if (!connected_) return;

        connected_ = false;
        if (IS_VALID_SOCKET(sock_)) {
            // Shutdown both Send/Receive to unblock recv()
            #ifdef _WIN32
                shutdown(sock_, SD_BOTH);
            #else
                shutdown(sock_, SHUT_RDWR);
            #endif
            CLOSE_SOCKET(sock_);
            sock_ = INVALID_SOCKET;
        }

        if (receiver_thread_.joinable()) {
            receiver_thread_.join();
        }
    }

    bool TcpClient::send_message(const std::string& msg) {
        if (!connected_) return false;

        // NDJSON framing: append newline if not present
        std::string payload = msg;
        if (payload.empty() || payload.back() != '\n') {
            payload += '\n';
        }

        // Send
        int sent = send(sock_, payload.c_str(), payload.length(), 0);
        if (sent == SOCKET_ERROR) {
            std::cerr << "Send failed: " << SOCKET_ERROR_CODE << std::endl;
            return false;
        }
        return true;
    }

    void TcpClient::receive_loop() {
        const int BUFFER_SIZE = 4096;
        std::vector<char> buffer(BUFFER_SIZE);
        std::string accumulation;

        while (connected_) {
            int bytes_read = recv(sock_, buffer.data(), BUFFER_SIZE, 0);
            
            if (bytes_read > 0) {
                // Append to accumulation
                accumulation.append(buffer.data(), bytes_read);

                // Process NDJSON (split by newline)
                size_t pos = 0;
                while ((pos = accumulation.find('\n')) != std::string::npos) {
                    std::string line = accumulation.substr(0, pos);
                    if (!line.empty() && on_message_) {
                        on_message_(line);
                    }
                    accumulation.erase(0, pos + 1);
                }
            } else if (bytes_read == 0) {
                // Server closed connection
                std::cout << "[Network] Server closed connection." << std::endl;
                connected_ = false;
                break;
            } else {
                // Error (or shutdown called)
                if (connected_) { // Only log if we didn't initiate shutdown
                     // std::cerr << "Recv failed: " << SOCKET_ERROR_CODE << std::endl;
                }
                connected_ = false;
                break;
            }
        }
    }

}
