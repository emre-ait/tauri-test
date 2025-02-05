# Gerekli kütüphanelerin kurulumu:
# Ubuntu için:
# sudo apt-get install liblcms2-dev python3-pip
#
# Windows için:
# Windows'ta 
# 1. MSYS2'yi indirin ve kurun: https://www.msys2.org/
# 2. Kurulum tamamlandıktan sonra MSYS2 terminalini açın.
# 3. MSYS2'yi güncelleyin:
#    pacman -Syu
#    (Eğer terminal kapanırsa tekrar açıp aşağıdaki komutu çalıştırın)
#    pacman -Su
#
# 4. Mingw-w64 araçlarını ve lcms2 kütüphanesini yükleyin:
#    pacman -S mingw-w64-x86_64-lcms2
#
# 5. DLL'nin yüklenip yüklenmediğini kontrol edin:
#    ls /mingw64/bin | grep lcms
#    (Büyük ihtimalle `liblcms2-2.dll` ve `liblcms2_fast_float.dll` görünecektir.)
#
# Kurulumun kontrolü:
# Ubuntu için:
# ldconfig -p | grep lcms2  # lcms2 kütüphanesinin varlığını kontrol eder
#
# Windows için:
# where lcms2.dll  # lcms2.dll'in konumunu gösterir

from PIL import Image
import numpy as np
from ctypes import *
import tifffile
import os  # En üstteki import kısmına ekleyin

# Global yol tanımlamaları - main() fonksiyonundan önce ekleyin
# Proje kök dizinini belirle
PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))

# ICC profil yolları
ICC_PROFILES_DIR = os.path.join(PROJECT_ROOT, 'resources', 'icc_profiles')
RGB_PROFILE = os.path.join(ICC_PROFILES_DIR, 'sRGB.icc')
CMYK_PROFILE = os.path.join(ICC_PROFILES_DIR, 'output_CMYK.icc')

# Görüntü yolları
IMAGES_DIR = os.path.join(PROJECT_ROOT, 'resources', 'images')
INPUT_IMAGE = os.path.join(IMAGES_DIR, 'test.png')
OUTPUT_IMAGE = os.path.join(IMAGES_DIR, 'output_test3.tiff')

# lcms2 kütüphanesini yükle
lcms2 = cdll.LoadLibrary('/lib/x86_64-linux-gnu/liblcms2.so')
# Windows için: lcms2 = cdll.LoadLibrary('lcms2.dll')


# Fonksiyon dönüş tiplerini tanımla
lcms2.cmsOpenProfileFromFile.restype = c_void_p
lcms2.cmsCreateTransform.restype = c_void_p
lcms2.cmsDoTransform.restype = None
lcms2.cmsSaveProfileToMem.restype = c_bool

# lcms2 sabitleri - C++ kodundan alınan doğru değerler
TYPE_RGB_16 = 0x4001a     # 262170 (C++ çıktısından)
TYPE_CMYK_16 = 0x60022    # 393250 (C++ çıktısından)
INTENT_PERCEPTUAL = 0

# Debug için format değerlerini yazdıralım
print(f"Using RGB format: {hex(TYPE_RGB_16)}")
print(f"Using CMYK format: {hex(TYPE_CMYK_16)}")

# lcms2 error handler ekleyin
ERROR_HANDLER_TYPE = CFUNCTYPE(None, c_void_p, c_uint32, c_char_p)

@ERROR_HANDLER_TYPE
def error_handler(contextID, errorcode, error_text):
    print(f"LCMS2 Error: {error_text.decode()}")

lcms2.cmsSetLogErrorHandler(error_handler)

# Başta eklenecek/değiştirilecek tanımlamalar
lcms2.cmsOpenProfileFromFile.argtypes = [c_char_p, c_char_p]
lcms2.cmsCreateTransform.argtypes = [c_void_p, c_uint32, c_void_p, c_uint32, c_uint32, c_uint32]
lcms2.cmsDoTransform.argtypes = [c_void_p, c_void_p, c_void_p, c_uint32]
lcms2.cmsSaveProfileToMem.argtypes = [c_void_p, c_void_p, POINTER(c_uint32)]
lcms2.cmsCloseProfile.argtypes = [c_void_p]
lcms2.cmsDeleteTransform.argtypes = [c_void_p]

def print_profile_info(profile_handle):
    if profile_handle:
        print(f"Profil başarıyla yüklendi: {profile_handle}")
    else:
        print("Profil yüklenemedi!")

class ImageColorConverter:
    def __init__(self):
        self.h_in_profile = None
        self.h_out_profile = None
        self.h_transform = None

    def initialize(self, rgb_profile_path, cmyk_profile_path):
        try:
            # Profil yükleme işlemini debug et
            self.h_in_profile = lcms2.cmsOpenProfileFromFile(rgb_profile_path.encode(), b"r")
            print_profile_info(self.h_in_profile)
            
            self.h_out_profile = lcms2.cmsOpenProfileFromFile(cmyk_profile_path.encode(), b"r")
            print_profile_info(self.h_out_profile)
            

            # Transform oluşturma işlemini debug et
            in_ptr = c_void_p(self.h_in_profile)
            out_ptr = c_void_p(self.h_out_profile)
            in_fmt = c_uint32(TYPE_RGB_16)
            out_fmt = c_uint32(TYPE_CMYK_16)
            intent = c_uint32(INTENT_PERCEPTUAL)
            flags = c_uint32(0)  # Flags'i sıfıra çevirelim

            print(f"Debug - Input Profile: {in_ptr.value}")
            print(f"Debug - Output Profile: {out_ptr.value}")
            print(f"Debug - Input Format: {hex(in_fmt.value)}")
            print(f"Debug - Output Format: {hex(out_fmt.value)}")

            transform = lcms2.cmsCreateTransform(
                in_ptr, 
                in_fmt, 
                out_ptr, 
                out_fmt, 
                intent,
                flags)

            if transform:
                self.h_transform = transform
                print("Transform başarıyla oluşturuldu!")
                return True
            else:
                print("Transform oluşturulamadı!")
                self.cleanup()
                return False
            
        except Exception as e:
            print(f"Initialize hatası: {str(e)}")
            self.cleanup()
            return False

    def convert_image(self, input_path, output_path):
        try:
            with Image.open(input_path) as img:
                img = img.convert('RGB')
                width, height = img.size
                
                # RGB verilerini hazırla
                rgb_data = np.array(img)
                rgb16_data = (rgb_data * 257).astype(np.uint16)
                rgb16_data = np.ascontiguousarray(rgb16_data)  # Bellek düzenini garantile
                
                # CMYK buffer'ı hazırla
                cmyk_data = np.zeros((height, width, 4), dtype=np.uint16)
                cmyk_data = np.ascontiguousarray(cmyk_data)
                
                # Dönüşümü gerçekleştir
                lcms2.cmsDoTransform(
                    c_void_p(self.h_transform),
                    rgb16_data.ctypes.data_as(c_void_p),
                    cmyk_data.ctypes.data_as(c_void_p),
                    width * height
                )
                
                # ICC profilini al
                profile_size = c_uint32()
                if not lcms2.cmsSaveProfileToMem(c_void_p(self.h_out_profile), None, byref(profile_size)):
                    print("ICC profil boyutu alınamadı!")
                    return False
                    
                profile_buffer = create_string_buffer(profile_size.value)
                if not lcms2.cmsSaveProfileToMem(c_void_p(self.h_out_profile), profile_buffer, byref(profile_size)):
                    print("ICC profil verisi alınamadı!")
                    return False
                    
                profile_data = bytes(profile_buffer.raw[:profile_size.value])
                
                # TIFF olarak kaydet - extratags düzeltildi
                tifffile.imwrite(
                    output_path,
                    cmyk_data,
                    photometric='SEPARATED',
                    compression=None,
                    resolution=(300, 300),
                    metadata=None,
                    description=None,
                    software=None,
                    extratags=[(34675, 7, profile_size.value, profile_data)]  # ICC profil tag'i düzeltildi
                )
                # Dosyayı kaydettikten sonra kontrol işlemleri
                print("\nKaydedilen TIFF dosyası analizi:")
                with tifffile.TiffFile(output_path) as tif:
                    # ICC profil bilgilerini kontrol et
                    icc_profile = None
                    for tag in tif.pages[0].tags:
                        if tag.name == 'ICCProfile':
                            icc_profile = tag.value
                            print("ICC Profili bulundu!")
                            
                    # Görüntü bilgilerini yazdır
                    page = tif.pages[0]
                    print(f"\nTIFF Metadata:")
                    print(f"Photometric: {page.photometric}")
                    print(f"Compression: {page.compression}")
                    print(f"Image Shape: {page.shape}")
                    print(f"Samples per pixel: {page.samplesperpixel}")
                    print(f"Bits per sample: {page.bitspersample}")
                    print(f"ICC Profile mevcut: {'Evet' if icc_profile else 'Hayır'}")
                    
                    # Görüntü verilerini oku ve analiz et
                    img_data = page.asarray()
                    print(f"\nGörüntü array şekli: {img_data.shape}")
                    print(f"Veri tipi: {img_data.dtype}")
                    
                    # İlk pixelin değerlerini göster
                    first_pixel = img_data[0, 0]
                    print("\nİlk pixel değerleri (16-bit):")
                    channel_names = ['Cyan', 'Magenta', 'Yellow', 'Key (Black)']
                    for i, (name, value) in enumerate(zip(channel_names, first_pixel)):
                        percentage = (value / 65535.0) * 100
                        print(f"{name}: {value} ({percentage:.2f}%)")
                    
                    # Minimum ve maksimum değerleri kontrol et
                    print("\nKanal istatistikleri:")
                    for i, name in enumerate(channel_names):
                        channel = img_data[..., i]
                        print(f"{name}:")
                        print(f"  Min: {channel.min()} ({(channel.min() / 65535.0) * 100:.2f}%)")
                        print(f"  Max: {channel.max()} ({(channel.max() / 65535.0) * 100:.2f}%)")
                        print(f"  Ortalama: {channel.mean():.2f} ({(channel.mean() / 65535.0) * 100:.2f}%)")
                return True
                
        except Exception as e:
            print(f"Hata oluştu: {str(e)}")
            return False

    def cleanup(self):
        if self.h_transform:
            lcms2.cmsDeleteTransform(c_void_p(self.h_transform))
        if self.h_in_profile:
            lcms2.cmsCloseProfile(c_void_p(self.h_in_profile))
        if self.h_out_profile:
            lcms2.cmsCloseProfile(c_void_p(self.h_out_profile))
        self.h_transform = None
        self.h_in_profile = None
        self.h_out_profile = None

    def __del__(self):
        self.cleanup()


def main():
    converter = ImageColorConverter()
    
    # Global yolları kullan
    if not converter.initialize(RGB_PROFILE, CMYK_PROFILE):
        print("Converter başlatılamadı!")
        return 1

    # Global yolları kullan
    if converter.convert_image(INPUT_IMAGE, OUTPUT_IMAGE):
        print(f"Dönüşüm başarılı! Dosya kaydedildi: {OUTPUT_IMAGE}")
    else:
        print("Dönüşüm sırasında hata oluştu!")
        return 1

    return 0

if __name__ == "__main__":
    main() 