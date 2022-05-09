#include <iostream>
#include <stdlib.h>

using namespace std;

int main(int argc, char *argv[]) {
    string ipAddr = argv[1];
    int portNum = stoi(argv[2]);
    system("cargo build");
    string runCommand = "cargo run " + ipAddr + " " + to_string(portNum);
    system(runCommand.c_str());
    return 0;
}