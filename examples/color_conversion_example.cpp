#include "color/ColorConverter.hpp"
#include "color/ColorTypes.hpp"
#include <iostream>
#include <vector>

int main() {
    ColorConverter converter;
    
    // ICC profil yolları - projenizin resources klasörüne göre ayarlayın
    std::string rgbProfile = "../resources/icc_profiles/sRGB.icc";
    std::string cmykProfile = "../resources/icc_profiles/output_CMYK.icc";
    
    if (!converter.initialize(rgbProfile, cmykProfile)) {
        std::cerr << "Converter başlatılamadı!" << std::endl;
        return 1;
    }

    // 16-bit test verisi (0-65535 arası değerler)
    std::vector<RGB16> rgbPixels = {
        {0, 0, 0},         // Siyah (R: 0, G: 0, B: 0)
        {65535, 0, 0},     // Kırmızı
        {0, 65535, 0},     // Yeşil
        {0, 0, 65535}      // Mavi
    };

    std::vector<CMYK16> cmykPixels(rgbPixels.size());

    // Dönüşümü gerçekleştir
    if (converter.convertRGBtoCMYK(
        reinterpret_cast<const uint16_t*>(rgbPixels.data()),
        reinterpret_cast<uint16_t*>(cmykPixels.data()),
        rgbPixels.size())) {
        
        std::cout << "Dönüşüm başarılı!" << std::endl;
        
        // İlk pikselin değerlerini göster (0-65535 arası)
        std::cout << "İlk piksel CMYK değerleri (16-bit): "
                  << cmykPixels[0].c << ", "
                  << cmykPixels[0].m << ", "
                  << cmykPixels[0].y << ", "
                  << cmykPixels[0].k << std::endl;
                  
        // Yüzdelik değerleri göster
        std::cout << "İlk piksel CMYK değerleri (%): "
                  << (cmykPixels[0].c * 100.0 / 65535) << "%, "
                  << (cmykPixels[0].m * 100.0 / 65535) << "%, "
                  << (cmykPixels[0].y * 100.0 / 65535) << "%, "
                  << (cmykPixels[0].k * 100.0 / 65535) << "%" << std::endl;
    }

    return 0;
} 