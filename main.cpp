#include <iostream>
#include <stdlib.h>

using namespace std;

int main() {
    cout << "TEST TEST" << endl;
    system("cargo build");
    system("cargo run");
    return 0;
}