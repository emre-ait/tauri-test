#include "color/ColorConverter.hpp"
#include <iostream>

ColorConverter::ColorConverter() 
    : hInProfile(nullptr), hOutProfile(nullptr), hTransform(nullptr) {}

ColorConverter::~ColorConverter() {
    if (hTransform) cmsDeleteTransform(hTransform);
    if (hInProfile) cmsCloseProfile(hInProfile);
    if (hOutProfile) cmsCloseProfile(hOutProfile);
}

bool ColorConverter::initialize(const std::string& rgbProfilePath, 
                              const std::string& cmykProfilePath) {
    hInProfile = cmsOpenProfileFromFile(rgbProfilePath.c_str(), "r");
    if (!hInProfile) {
        std::cerr << "RGB profili yüklenemedi: " << rgbProfilePath << std::endl;
        return false;
    }

    hOutProfile = cmsOpenProfileFromFile(cmykProfilePath.c_str(), "r");
    if (!hOutProfile) {
        std::cerr << "CMYK profili yüklenemedi: " << cmykProfilePath << std::endl;
        return false;
    }

    hTransform = cmsCreateTransform(
        hInProfile,
        TYPE_RGB_16,        // 16-bit RGB
        hOutProfile,
        TYPE_CMYK_16,       // 16-bit CMYK
        INTENT_PERCEPTUAL,
        cmsFLAGS_BLACKPOINTCOMPENSATION | 
        cmsFLAGS_HIGHRESPRECALC    // Yüksek hassasiyet için
    );

    if (!hTransform) {
        std::cerr << "Transform oluşturulamadı!" << std::endl;
        return false;
    }

    return true;
}

bool ColorConverter::convertRGBtoCMYK(const uint16_t* rgbData, 
                                    uint16_t* cmykData, 
                                    size_t pixelCount) {
    if (!hTransform) {
        std::cerr << "Transform henüz oluşturulmamış!" << std::endl;
        return false;
    }
    
    cmsDoTransform(hTransform, rgbData, cmykData, pixelCount);
    return true;
} 