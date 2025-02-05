#include "math_lib.h"
#include <iostream>

namespace math_lib {
    double Calculator::square_root(double x) {
        if (x < 0) {
            std::cout << "Warning: Cannot calculate square root of negative number" << std::endl;
            return 0.0;
        }
        return std::sqrt(x);
    }

    double Calculator::power(double base, double exponent) {
        return std::pow(base, exponent);
    }

    double Calculator::absolute(double x) {
        return std::abs(x);
    }
} 