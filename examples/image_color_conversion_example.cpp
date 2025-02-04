#include <iostream>
#include <vector>
#include <string>
#include "lcms2.h"
#include <tiffio.h>

// stb_image kütüphanelerini dahil et
#define STB_IMAGE_IMPLEMENTATION
#include "/home/emre/denemeler/tauri-test/include/stb_image.h"
#define STB_IMAGE_WRITE_IMPLEMENTATION
#include "/home/emre/denemeler/tauri-test/include/stb_image_write.h"

class ImageColorConverter {
public:
    bool initialize(const std::string& rgbProfilePath, const std::string& cmykProfilePath) {
        // ICC profillerini yükle
        hInProfile = cmsOpenProfileFromFile(rgbProfilePath.c_str(), "r");
        hOutProfile = cmsOpenProfileFromFile(cmykProfilePath.c_str(), "r");
        
        if (!hInProfile || !hOutProfile) {
            cleanup();
            return false;
        }

        // Dönüşüm için transform oluştur
        hTransform = cmsCreateTransform(
            hInProfile,
            TYPE_RGB_16,
            hOutProfile,
            TYPE_CMYK_16,
            INTENT_PERCEPTUAL,
            0);

        if (!hTransform) {
            cleanup();
            return false;
        }

        return true;
    }

    bool convertImage(const std::string& inputPath, const std::string& outputPath) {
        int width, height, channels;
        std::cout << "Input path: " << inputPath << std::endl;
        std::cout << "Output path: " << outputPath << std::endl;
        // Resmi yükle
        uint8_t* inputData = stbi_load(inputPath.c_str(), &width, &height, &channels, 3);
        if (!inputData) {
            std::cerr << "Resim yüklenemedi: " << inputPath << std::endl;
            return false;
        }

        // RGB 8-bit'ten RGB 16-bit'e dönüştür
        std::vector<uint16_t> rgb16Data(width * height * 3);
        for (int i = 0; i < width * height * 3; ++i) {
            rgb16Data[i] = static_cast<uint16_t>(inputData[i] * 257); // 8-bit to 16-bit
        }

        // CMYK çıktı verisi için buffer (16-bit)
        std::vector<uint16_t> cmykData(width * height * 4);

        // Renk dönüşümünü gerçekleştir
        cmsDoTransform(hTransform, rgb16Data.data(), cmykData.data(), width * height);

        // TIFF olarak kaydet (16-bit CMYK)
        TIFF* tif = TIFFOpen(outputPath.c_str(), "w");
        if (!tif) {
            std::cerr << "TIFF dosyası oluşturulamadı" << std::endl;
            stbi_image_free(inputData);
            return false;
        }

        // TIFF parametrelerini ayarla
        TIFFSetField(tif, TIFFTAG_IMAGEWIDTH, width);
        TIFFSetField(tif, TIFFTAG_IMAGELENGTH, height);
        TIFFSetField(tif, TIFFTAG_SAMPLESPERPIXEL, 4); // CMYK
        TIFFSetField(tif, TIFFTAG_BITSPERSAMPLE, 16);  // 16-bit
        TIFFSetField(tif, TIFFTAG_ORIENTATION, ORIENTATION_TOPLEFT);
        TIFFSetField(tif, TIFFTAG_PLANARCONFIG, PLANARCONFIG_CONTIG);
        TIFFSetField(tif, TIFFTAG_PHOTOMETRIC, PHOTOMETRIC_SEPARATED); // CMYK
        TIFFSetField(tif, TIFFTAG_COMPRESSION, COMPRESSION_LZW);       // LZW sıkıştırma

        // ICC profilini gömme
        cmsUInt32Number profileSize;
        cmsSaveProfileToMem(hOutProfile, NULL, &profileSize); // Profil boyutunu al
        void* profileData = malloc(profileSize);
        cmsSaveProfileToMem(hOutProfile, profileData, &profileSize);
        TIFFSetField(tif, TIFFTAG_ICCPROFILE, profileSize, profileData);

        // CMYK verilerini yaz
        for (int row = 0; row < height; row++) {
            if (TIFFWriteScanline(tif, &cmykData[row * width * 4], row, 0) < 0) {
                std::cerr << "TIFF yazma hatası" << std::endl;
                TIFFClose(tif);
                free(profileData);
                stbi_image_free(inputData);
                return false;
            }
        }

        TIFFClose(tif);
        free(profileData);
        stbi_image_free(inputData);
        return true;
    }

    ~ImageColorConverter() {
        cleanup();
    }

private:
    cmsHPROFILE hInProfile = nullptr;
    cmsHPROFILE hOutProfile = nullptr;
    cmsHTRANSFORM hTransform = nullptr;

    void cleanup() {
        if (hTransform) cmsDeleteTransform(hTransform);
        if (hInProfile) cmsCloseProfile(hInProfile);
        if (hOutProfile) cmsCloseProfile(hOutProfile);
        hTransform = nullptr;
        hInProfile = nullptr;
        hOutProfile = nullptr;
    }
};

int main() {
    ImageColorConverter converter;
    
    // ICC profil yolları
    std::string rgbProfile = "/home/emre/denemeler/tauri-test/resources/icc_profiles/sRGB.icc";
    std::string cmykProfile = "/home/emre/denemeler/tauri-test/resources/icc_profiles/output_CMYK.icc";
    
    if (!converter.initialize(rgbProfile, cmykProfile)) {
        std::cerr << "Converter başlatılamadı!" << std::endl;
        return 1;
    }

    // Dönüşümü gerçekleştir
    std::string inputImage = "/home/emre/denemeler/tauri-test/resources/images/test_9000.png";  // veya .jpg
    std::string outputImage = "/home/emre/denemeler/tauri-test/resources/images/output_test2.tiff";
    
    if (converter.convertImage(inputImage, outputImage)) {
        std::cout << "Dönüşüm başarılı! Dosya kaydedildi: " << outputImage << std::endl;
    } else {
        std::cerr << "Dönüşüm sırasında hata oluştu!" << std::endl;
        return 1;
    }

    return 0;
} 