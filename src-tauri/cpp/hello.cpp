#include "hello.h"
#include <iostream>

void say_hello() {
    std::cout << "Hello World from C++!" << std::endl;
}

double add(double a, double b) {
    std::cout << "C++: Adding " << a << " + " << b << std::endl;
    return a + b;
}

double subtract(double a, double b) {
    std::cout << "C++: Subtracting " << a << " - " << b << std::endl;
    return a - b;
}

double multiply(double a, double b) {
    std::cout << "C++: Multiplying " << a << " * " << b << std::endl;
    return a * b;
}

double divide(double a, double b) {
    std::cout << "C++: Dividing " << a << " / " << b << std::endl;
    if (b == 0) {
        std::cout << "Error: Division by zero!" << std::endl;
        return 0.0;
    }
    return a / b;
}

// New math functions implementation
double calculate_sqrt(double x) {
    std::cout << "C++: Calculating square root of " << x << std::endl;
    return math_lib::Calculator::square_root(x);
}

double calculate_power(double base, double exp) {
    std::cout << "C++: Calculating " << base << " ^ " << exp << std::endl;
    return math_lib::Calculator::power(base, exp);
}

double calculate_abs(double x) {
    std::cout << "C++: Calculating absolute value of " << x << std::endl;
    return math_lib::Calculator::absolute(x);
} 