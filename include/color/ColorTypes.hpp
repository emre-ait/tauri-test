#ifndef COLOR_TYPES_HPP
#define COLOR_TYPES_HPP

#include <cstdint>

struct RGB16 {
    uint16_t r, g, b;
};

struct CMYK16 {
    uint16_t c, m, y, k;
};

#endif // COLOR_TYPES_HPP 