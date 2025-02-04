# Tauri + Vue + TypeScript.

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

[![publish](https://github.com/Fenish/tauri-example/actions/workflows/build.yaml/badge.svg)](https://github.com/Fenish/tauri-example/actions/workflows/build.yaml)

## Gereksinimler

### Ubuntu/Debian için:
```bash
# Temel geliştirme araçları
sudo apt-get install build-essential cmake pkg-config

# LittleCMS2 kütüphanesi
sudo apt-get install liblcms2-dev

# TIFF kütüphanesi
sudo apt-get install libtiff-dev

# Tauri gereksinimleri
sudo apt install libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf
```

## Usage

### Development

```bash
pnpm dev
```

or

```bash
pnpm tauri dev
```

### Build

```bash
pnpm tauri build
```

# RIP Projesi

Bu proje, dijital baskı makineleri için RGB'den CMYK'ya renk dönüşümü yapan bir RIP (Raster Image Processor) sistemidir.

## Özellikler
- RGB -> CMYK dönüşümü
- 16-bit renk derinliği
- ICC profil desteği
- TIFF formatında kaydetme