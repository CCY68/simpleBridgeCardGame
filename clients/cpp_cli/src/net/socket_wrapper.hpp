#ifndef SOCKET_WRAPPER_HPP
#define SOCKET_WRAPPER_HPP

#include <string>
#include <iostream>

// Platform detection and includes
#ifdef _WIN32
    #include <winsock2.h>
    #include <ws2tcpip.h>
    #pragma comment(lib, "ws2_32.lib")
    
    using socket_t = SOCKET;
    #define IS_VALID_SOCKET(s) ((s) != INVALID_SOCKET)
    #define CLOSE_SOCKET(s) closesocket(s)
    #define SOCKET_ERROR_CODE WSAGetLastError()
#else
    #include <sys/socket.h>
    #include <arpa/inet.h>
    #include <unistd.h>
    #include <netdb.h>
    #include <fcntl.h>
    
    using socket_t = int;
    #define IS_VALID_SOCKET(s) ((s) >= 0)
    #define CLOSE_SOCKET(s) close(s)
    #define SOCKET_ERROR_CODE errno
    #define INVALID_SOCKET -1
    #define SOCKET_ERROR -1
#endif

namespace net {

    // Initialize Winsock on Windows, do nothing on Linux
    inline bool initialize() {
        #ifdef _WIN32
            WSADATA wsaData;
            int res = WSAStartup(MAKEWORD(2, 2), &wsaData);
            if (res != 0) {
                std::cerr << "WSAStartup failed: " << res << std::endl;
                return false;
            }
        #endif
        return true;
    }

    // Cleanup Winsock on Windows
    inline void cleanup() {
        #ifdef _WIN32
            WSACleanup();
        #endif
    }

}

#endif // SOCKET_WRAPPER_HPP
