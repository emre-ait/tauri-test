cmake_minimum_required(VERSION 3.10)
project(ColorConversionExamples)

# C++17 standardını kullan
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# pkg-config'i kullan
find_package(PkgConfig REQUIRED)
pkg_check_modules(LCMS2 REQUIRED lcms2)
pkg_check_modules(TIFF REQUIRED libtiff-4)

# Include dizinlerini ekle
include_directories(${CMAKE_SOURCE_DIR}/include)
include_directories(${LCMS2_INCLUDE_DIRS})
include_directories(${TIFF_INCLUDE_DIRS})

# Kaynak dosyaları topla
file(GLOB_RECURSE SOURCES "src/*.cpp")

# Kütüphaneyi oluştur
add_library(color_converter STATIC ${SOURCES})
target_link_libraries(color_converter ${LCMS2_LIBRARIES} ${TIFF_LIBRARIES})

# Örnekleri oluştur
add_executable(color_example examples/color_conversion_example.cpp)
add_executable(image_example examples/image_color_conversion_example.cpp)

# Örnekleri kütüphaneye bağla
target_link_libraries(color_example color_converter)
target_link_libraries(image_example color_converter) 