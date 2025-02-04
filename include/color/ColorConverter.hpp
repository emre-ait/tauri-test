#ifndef COLOR_CONVERTER_HPP
#define COLOR_CONVERTER_HPP

#include <lcms2.h>
#include <string>
#include <cstdint>

class ColorConverter {
public:
    ColorConverter();
    ~ColorConverter();

    bool initialize(const std::string& rgbProfilePath, 
                   const std::string& cmykProfilePath);
    
    bool convertRGBtoCMYK(const uint16_t* rgbData, 
                         uint16_t* cmykData, 
                         size_t pixelCount);

private:
    cmsHPROFILE hInProfile;
    cmsHPROFILE hOutProfile;
    cmsHTRANSFORM hTransform;
};

#endif // COLOR_CONVERTER_HPP 