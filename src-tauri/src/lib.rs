use serde::{Deserialize, Serialize};
use image::{ImageFormat, GenericImageView, RgbImage, Rgb};
use std::io::{Cursor, Read, Write, Seek, SeekFrom};
use std::fs::File;
use byteorder::{LittleEndian, ReadBytesExt};
use phf::phf_map;

#[derive(Debug, Serialize, Deserialize)]
struct ImageData {
    data: Vec<u8>,
    image_type: String,
    width: f64,
    height: f64,
    filename: String,
}

#[derive(Debug, Serialize)]
struct ProcessedImageData {
    width: f64,
    height: f64,
    resolution: (f64, f64),
    size_bytes: usize,
    format: String,
    detected_format: String,
}

// MIF File Structures
#[derive(Debug)]
struct MifHeader {
    file_version: String,
    version_id: i32,
    variant_count: i16,
    active_variant: i16,
    flags: i16,
    design_name: String,
    design_file_name: String,
    design_type: String,
    pc: i16,
    parameters: Vec<String>,
    password: Vec<u8>,
    repeat_mode: i16,
    repeat_dir: i16,
    repeat_offset: i32,
    tags: Vec<HeaderTag>,
}

const COLOR_TAG: &str = "C01";  // size 3

#[derive(Debug)]
struct MifColor {
    tag: String,
    red: u16,
    green: u16,
    blue: u16,
    color_type: i32,  // Changed from u8 to i32 to match Python
    l: u16,
    a: u16,
    b: u16,
    name: String,
    description: String,
    extra_datasize: i32,
}

impl MifColor {
    fn read(mfile: &mut MifReader) -> Result<Self, String> {
        let tag = mfile.read_tag(4)?;
        if tag != COLOR_TAG {
            return Err("ColorSection tag is not correct".to_string());
        }

        let color = MifColor {
            tag,
            red: mfile.read_uint16()?,
            green: mfile.read_uint16()?,
            blue: mfile.read_uint16()?,
            l: mfile.read_uint16()?,
            a: mfile.read_uint16()?,
            b: mfile.read_uint16()?,
            color_type: mfile.read_int32()?,
            name: mfile.read_string()?,
            description: mfile.read_string()?,
            extra_datasize: mfile.read_int32()?,
        };

        Ok(color)
    }

    fn to_rgb(&self) -> [u8; 3] {
        match self.color_type {
            0 => {
                // LAB to RGB conversion
                // For now, using simple conversion. We'll need proper LAB->RGB conversion
                let l = (self.l >> 8) as f32 / 255.0 * 100.0;
                let a = ((self.a >> 8) as f32 - 128.0) / 127.0 * 100.0;
                let b = ((self.b >> 8) as f32 - 128.0) / 127.0 * 100.0;
                
                // Placeholder conversion - needs proper LAB->RGB implementation
                [(l * 2.55) as u8, 
                 ((a + 128.0) * 1.0) as u8, 
                 ((b + 128.0) * 1.0) as u8]
            },
            1 => {
                // Direct RGB values
                [(self.red >> 8) as u8, 
                 (self.green >> 8) as u8, 
                 (self.blue >> 8) as u8]
            },
            _ => [0, 0, 0],
        }
    }

    fn write(&self, writer: &mut MifWriter) -> Result<(), String> {
        writer.write_tag(&self.tag, 4)?;
        writer.write_uint16(self.red)?;
        writer.write_uint16(self.green)?;
        writer.write_uint16(self.blue)?;
        writer.write_uint16(self.l)?;
        writer.write_uint16(self.a)?;
        writer.write_uint16(self.b)?;
        writer.write_int32(self.color_type)?;
        writer.write_string(&self.name)?;
        writer.write_string(&self.description)?;
        writer.write_int32(self.extra_datasize)?;
        Ok(())
    }
}

#[derive(Debug)]
struct MifChannel {
    width: u32,
    height: u32,
    data: Vec<u8>,
    name: String,
    visibility: bool,
    opacity: f32,
    color: MifColor,
}

#[derive(Debug)]
struct MifVariant {
    name: String,
    fabric_color: MifColor,
    channel_specs: Vec<MifChannelSpec>,
}

#[derive(Debug)]
struct MifChannelSpec {
    name: String,
    visibility: bool,
    opacity: f32,
    color: MifColor,
}

#[derive(Debug)]
struct MifLayer {
    channel_count: u32,
    channels: Vec<MifChannel>,
}

#[derive(Debug)]
struct MifFile {
    header: MifHeader,
    variants: Vec<MifVariant>,
    layers: Vec<MifLayer>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MifOpenRequest {
    file_path: String,
}

/// Render using native blending in CMY color space
fn render_native(dest: &mut RgbImage, source: &[u8], source_color: [u8; 3], channel_opacity: f32) {
    let (width, height) = dest.dimensions();
    
    // Iterate over each pixel
    for y in 0..height {
        for x in 0..width {
            let pixel = dest.get_pixel(x, y);
            
            // Convert RGB to CMY (convert to f32 first)
            let mut np_c = 255.0 - pixel[0] as f32;
            let mut np_m = 255.0 - pixel[1] as f32;
            let mut np_y = 255.0 - pixel[2] as f32;
            
            // Convert source color to CMY
            let color_c = 255.0 - source_color[0] as f32;
            let color_m = 255.0 - source_color[1] as f32;
            let color_y = 255.0 - source_color[2] as f32;
            
            // Get source value for this pixel
            let idx = (y * width + x) as usize;
            if idx >= source.len() { continue; }
            let blend = 255.0 - source[idx] as f32;
            
            // Calculate opacity
            let opacity = channel_opacity * blend / 128.0;
            
            // Dissolve dest channels
            let dissolve = blend * pixel[0] as f32 / 255.0;
            np_c = np_c * (255.0 - dissolve) / 255.0;
            np_m = np_m * (255.0 - dissolve) / 255.0;
            np_y = np_y * (255.0 - dissolve) / 255.0;
            
            // Apply opacity
            np_c = np_c * (255.0 - opacity) / 255.0;
            np_m = np_m * (255.0 - opacity) / 255.0;
            np_y = np_y * (255.0 - opacity) / 255.0;
            
            // Blend in channel with color
            np_c += (255.0 - np_c) * blend * color_c / (255.0 * 255.0);
            np_m += (255.0 - np_m) * blend * color_m / (255.0 * 255.0);
            np_y += (255.0 - np_y) * blend * color_y / (255.0 * 255.0);
            
            // Convert back to RGB and set pixel
            dest.put_pixel(x, y, Rgb([
                (255.0 - np_c) as u8,
                (255.0 - np_m) as u8,
                (255.0 - np_y) as u8,
            ]));
        }
    }
}

/// Render using additive blending in CMY color space
fn render_only_add(dest: &mut RgbImage, source: &[u8], source_color: [u8; 3], channel_opacity: f32) {
    let (width, height) = dest.dimensions();
    
    // Iterate over each pixel
    for y in 0..height {
        for x in 0..width {
            let pixel = dest.get_pixel(x, y);
            
            // Convert RGB to CMY (convert to f32 first)
            let mut np_c = 255.0 - pixel[0] as f32;
            let mut np_m = 255.0 - pixel[1] as f32;
            let mut np_y = 255.0 - pixel[2] as f32;
            
            // Convert source color to CMY
            let color_c = 255.0 - source_color[0] as f32;
            let color_m = 255.0 - source_color[1] as f32;
            let color_y = 255.0 - source_color[2] as f32;
            
            // Get source value for this pixel
            let idx = (y * width + x) as usize;
            if idx >= source.len() { continue; }
            let blend = 255.0 - source[idx] as f32;
            
            // Calculate opacity
            let opacity = channel_opacity * blend / 255.0;
            
            // Dissolve dest channels
            let dissolve = blend * pixel[0] as f32 / 255.0;
            np_c = np_c * (255.0 - dissolve) / 255.0;
            let dissolve = blend * pixel[1] as f32 / 255.0;
            np_m = np_m * (255.0 - dissolve) / 255.0;
            let dissolve = blend * pixel[2] as f32 / 255.0;
            np_y = np_y * (255.0 - dissolve) / 255.0;
            
            // Apply opacity
            np_c += (255.0 - np_c) * opacity / 128.0;
            np_m += (255.0 - np_m) * opacity / 128.0;
            np_y += (255.0 - np_y) * opacity / 128.0;
            
            // Blend in channel with color
            np_c = np_c - color_c * blend / 255.0;
            np_m = np_m - color_m * blend / 255.0;
            np_y = np_y - color_y * blend / 255.0;
            
            // Clamp negative values to 0
            np_c = np_c.max(0.0);
            np_m = np_m.max(0.0);
            np_y = np_y.max(0.0);
            
            // Convert back to RGB and set pixel
            dest.put_pixel(x, y, Rgb([
                (255.0 - np_c) as u8,
                (255.0 - np_m) as u8,
                (255.0 - np_y) as u8,
            ]));
        }
    }
}

fn create_interleaved_rgb(layer: &MifLayer, variant: &MifVariant, scale: u32) -> Result<RgbImage, String> {
    // Get dimensions from first channel
    if layer.channels.is_empty() {
        return Err("No channels found in layer".to_string());
    }

    let width = layer.channels[0].width / scale;
    let height = layer.channels[0].height / scale;
    
    // Create new RGB image
    let mut image = RgbImage::new(width, height);
    
    // Fill with fabric color
    let base_color = variant.fabric_color.to_rgb();
    for pixel in image.pixels_mut() {
        *pixel = Rgb(base_color);
    }
    
    // Process each channel
    for channel in &layer.channels {
        if !channel.visibility {
            // Skip invisible channels
            if channel.name == "C" || channel.name == "M" || channel.name == "Y" {
                continue;
            }
        }
        
        let color = channel.color.to_rgb();
        
        // Use render_native by default, could be made configurable
        render_native(&mut image, &channel.data, color, channel.opacity);
    }
    
    Ok(image)
}

#[tauri::command]
async fn process_mif_image(layer_index: usize, variant_index: usize, scale: u32) -> Result<Vec<u8>, String> {
    // TODO: Get actual MIF data from file
    // This is just a placeholder showing the structure
    let layer = MifLayer {
        channel_count: 1,
        channels: vec![
            MifChannel {
                width: 100,
                height: 100,
                data: vec![0; 10000],
                name: "Test".to_string(),
                visibility: true,
                opacity: 1.0,
                color: MifColor {
                    tag: String::new(),
                    red: 0,
                    green: 0,
                    blue: 0,
                    color_type: 0,
                    l: 0,
                    a: 0,
                    b: 0,
                    name: String::new(),
                    description: String::new(),
                    extra_datasize: 0,
                },
            }
        ],
    };
    
    let variant = MifVariant {
        name: "Default".to_string(),
        fabric_color: MifColor {
            tag: String::new(),
            red: 0,
            green: 0,
            blue: 0,
            color_type: 0,
            l: 0,
            a: 0,
            b: 0,
            name: String::new(),
            description: String::new(),
            extra_datasize: 0,
        },
        channel_specs: vec![],
    };
    
    let rgb_image = create_interleaved_rgb(&layer, &variant, scale)?;
    
    // Convert to PNG for transfer
    let mut bytes: Vec<u8> = Vec::new();
    rgb_image.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|e| format!("Failed to encode image: {}", e))?;
    
    Ok(bytes)
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn process_image(image_data: ImageData) -> Result<ProcessedImageData, String> {
    println!("Processing image: {}", image_data.filename);
    
    // Detect image format
    let format = image::guess_format(&image_data.data)
        .map_err(|e| format!("Failed to detect image format: {}", e))?;
    
    // Load image using the image crate
    let img = image::load_from_memory(&image_data.data)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    // Get image dimensions in pixels
    let (width_px, height_px) = img.dimensions();
    
    // Calculate resolution (assuming 96 DPI)
    let dpi = 96.0;
    let width_cm = (width_px as f64 / dpi) * 2.54;
    let height_cm = (height_px as f64 / dpi) * 2.54;
    
    // Create processed data response
    let processed_data = ProcessedImageData {
        width: width_cm,
        height: height_cm,
        resolution: (dpi, dpi),
        size_bytes: image_data.data.len(),
        format: image_data.image_type,
        detected_format: format_to_string(format),
    };
    
    println!("Detected format: {:?}", format);
    Ok(processed_data)
}

// Helper function to convert ImageFormat to String
fn format_to_string(format: ImageFormat) -> String {
    match format {
        ImageFormat::Png => "PNG".to_string(),
        ImageFormat::Jpeg => "JPEG".to_string(),
        ImageFormat::Gif => "GIF".to_string(),
        ImageFormat::WebP => "WebP".to_string(),
        ImageFormat::Tiff => "TIFF".to_string(),
        ImageFormat::Bmp => "BMP".to_string(),
        ImageFormat::Ico => "ICO".to_string(),
        ImageFormat::Tga => "TGA".to_string(),
        _ => "Unknown".to_string(),
    }
}

#[tauri::command]
async fn rotate_image(image_data: ImageData, _angle: i32) -> Result<Vec<u8>, String> {
    // Load image
    let img = image::load_from_memory(&image_data.data)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    // Rotate image
    let rotated = img.rotate90();
    
    // Convert back to bytes using a memory buffer
    let mut bytes = Vec::new();
    rotated.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|e| format!("Failed to encode image: {}", e))?;
    
    Ok(bytes)
}

const VARIANT002: &str = "VAR0002";
const VARIANT003: &str = "VAR0003";
const VARIANT004: &str = "VAR0004";
const VARIANT005: &str = "VAR0005";
const SIMPROPIDV17: &str = "SIM0001";
const SIMPROPIDV18: &str = "SIM0002";
const SIMPROPIDV19: &str = "SIM0003";

// MFile equivalent in Rust
struct MifReader {
    file: File,
}

impl MifReader {
    fn new(filepath: &str) -> Result<Self, String> {
        let file = File::open(filepath)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        Ok(MifReader { file })
    }

    fn read_bytes(&mut self, size: usize) -> Result<Vec<u8>, String> {
        let mut buffer = vec![0u8; size];
        self.file.read_exact(&mut buffer)
            .map_err(|e| format!("Failed to read bytes: {}", e))?;
        Ok(buffer)
    }

    fn read_int8(&mut self) -> Result<i8, String> {
        self.file.read_i8()
            .map_err(|e| format!("Failed to read i8: {}", e))
    }

    fn read_int16(&mut self) -> Result<i16, String> {
        self.file.read_i16::<LittleEndian>()
            .map_err(|e| format!("Failed to read i16: {}", e))
    }

    fn read_int32(&mut self) -> Result<i32, String> {
        self.file.read_i32::<LittleEndian>()
            .map_err(|e| format!("Failed to read i32: {}", e))
    }

    fn read_uint8(&mut self) -> Result<u8, String> {
        self.file.read_u8()
            .map_err(|e| format!("Failed to read u8: {}", e))
    }

    fn read_uint16(&mut self) -> Result<u16, String> {
        self.file.read_u16::<LittleEndian>()
            .map_err(|e| format!("Failed to read u16: {}", e))
    }

    fn read_uint32(&mut self) -> Result<u32, String> {
        self.file.read_u32::<LittleEndian>()
            .map_err(|e| format!("Failed to read u32: {}", e))
    }

    fn read_float(&mut self) -> Result<f32, String> {
        self.file.read_f32::<LittleEndian>()
            .map_err(|e| format!("Failed to read f32: {}", e))
    }

    fn read_boolean(&mut self) -> Result<bool, String> {
        let value = self.read_bytes(1)?;
        Ok(value[0] != 0)
    }

    fn read_tag(&mut self, size: usize) -> Result<String, String> {
        let bytes = self.read_bytes(size)?;
        let s = String::from_utf8_lossy(&bytes);
        Ok(s.trim_end_matches('\0').to_string())
    }

    fn read_string(&mut self) -> Result<String, String> {
        let length = self.read_int16()? as usize;
        let bytes = self.read_bytes(length)?;
        
        // Try UTF-8 first, fallback to Windows-1254 (cp1254)
        match String::from_utf8(bytes.clone()) {
            Ok(s) => Ok(s.trim_end_matches('\0').to_string()),
            Err(_) => {
                // Fallback to Windows-1254 encoding
                let s: String = bytes.iter()
                    .map(|&b| match WINDOWS_1254_TO_UTF8.get(&b) {
                        Some(&c) => c,
                        None => b as char,
                    })
                    .collect();
                Ok(s.trim_end_matches('\0').to_string())
            }
        }
    }

    fn jump(&mut self, offset: u64) -> Result<(), String> {
        self.file.seek(SeekFrom::Start(offset))
            .map_err(|e| format!("Failed to seek: {}", e))?;
        Ok(())
    }

    fn stream_position(&mut self) -> Result<u64, String> {
        self.file.stream_position()
            .map_err(|e| format!("Failed to get stream position: {}", e))
    }
}

// Windows-1254 to UTF-8 conversion map (partial)
static WINDOWS_1254_TO_UTF8: phf::Map<u8, char> = phf_map! {
    0x80u8 => '€',
    0x82u8 => '‚',
    0x8Cu8 => 'Œ',
    0x8Du8 => 'œ',
    0x8Eu8 => 'Ž',
    0x9Cu8 => 'œ',
    0x9Du8 => 'Ÿ',
    0x9Eu8 => 'ž',
    0xC0u8 => 'À',
    0xC1u8 => 'Á',
    0xC2u8 => 'Â',
    0xC3u8 => 'Ã',
    0xC4u8 => 'Ä',
    0xC5u8 => 'Å',
    0xC6u8 => 'Æ',
    0xC7u8 => 'Ç',
    0xC8u8 => 'È',
    0xC9u8 => 'É',
    0xCAu8 => 'Ê',
    0xCBu8 => 'Ë',
    0xCCu8 => 'Ì',
    0xCDu8 => 'Í',
    0xCEu8 => 'Î',
    0xCFu8 => 'Ï'
};

#[derive(Debug)]
struct HeaderTag {
    id: i16,
    size: i32,
    data: Vec<u8>,
}

const DESIGN_MAXPARAMETERCOUNT: i16 = 20;
const MIFFIDV19: &str = "MIFF001";  // size 7 version 1.9
const MIFFIDV20: &str = "MIFF002";  // size 7 version 2.0
const MIFFIDV28: &str = "MIFF028";  // size 7 version 3
const MIFFIDV30: &str = "MIFF030";  // size 7 version 3.0

#[derive(Debug)]
struct VariantHeaderSection {
    factory_name: String,
    tag: String,
    name: String,
    description: String,
    entry_date: String,
    design_name: String,
    design_type: String,
    fabric_type: String,
    print_type: String,
    channel_count1: i16,
    channel_count2: i16,
}

impl VariantHeaderSection {
    fn get_variant_version(&self) -> Result<i32, String> {
        match self.tag.as_str() {
            VARIANT002 => Ok(1),
            VARIANT003 => Ok(2),
            VARIANT004 => Ok(3),
            VARIANT005 => Ok(4),
            _ => Err("Unknown variant tag".to_string()),
        }
    }

    fn read(mfile: &mut MifReader) -> Result<Self, String> {
        let mut header = VariantHeaderSection {
            factory_name: String::new(),
            tag: String::new(),
            name: String::new(),
            description: String::new(),
            entry_date: String::new(),
            design_name: String::new(),
            design_type: String::new(),
            fabric_type: String::new(),
            print_type: String::new(),
            channel_count1: 0,
            channel_count2: 0,
        };

        header.tag = mfile.read_tag(8)?;
        let version = header.get_variant_version()?;

        if version < 4 {
            header.description = mfile.read_string()?;
            header.name = mfile.read_string()?;
            header.entry_date = mfile.read_string()?;
            header.design_name = mfile.read_string()?;
            header.design_type = mfile.read_string()?;
        } else {
            header.name = mfile.read_string()?;
            header.description = mfile.read_string()?;
            header.entry_date = mfile.read_string()?;
        }

        if version == 1 {
            header.print_type = mfile.read_string()?;
            header.factory_name = mfile.read_string()?;
        }

        header.channel_count1 = mfile.read_int16()?;
        header.channel_count2 = mfile.read_int16()?;

        Ok(header)
    }
}

impl MifHeader {
    fn get_version(&self) -> i32 {
        match self.file_version.as_str() {
            MIFFIDV19 => 1,
            MIFFIDV20 => 2,
            MIFFIDV28 => 3,
            MIFFIDV30 => 4,
            _ => -1,
        }
    }

    fn read_tags(&mut self, mfile: &mut MifReader) -> Result<(), String> {
        loop {
            let tag_size = mfile.read_int32()?;
            if tag_size == 0 {
                break;
            }
            
            let tag_id = mfile.read_int16()?;
            let tag_data = mfile.read_bytes(tag_size as usize)?;
            
            self.tags.push(HeaderTag {
                id: tag_id,
                size: tag_size,
                data: tag_data,
            });
        }
        Ok(())
    }

    fn read_v3_and_older(&mut self, mfile: &mut MifReader) -> Result<bool, String> {
        self.design_name = mfile.read_string()?;
        self.design_file_name = mfile.read_string()?;
        self.design_type = mfile.read_string()?;

        if self.version_id == 1 {
            self.pc = 4;
        } else {
            self.pc = mfile.read_int16()?;
            if self.pc <= 0 || self.pc > DESIGN_MAXPARAMETERCOUNT {
                let _temp_buffer = mfile.read_bytes(254)?;
                let pc = mfile.read_int16()?;
                if pc <= 0 || pc > DESIGN_MAXPARAMETERCOUNT {
                    return Ok(false);
                }
                self.file_version = MIFFIDV30.to_string();
                self.version_id = self.get_version();
            }
        }

        // Read parameters
        for _ in 0..self.pc {
            self.parameters.push(mfile.read_string()?);
        }

        // Read password if not version 4
        if self.version_id != 4 {
            let pass_len = mfile.read_int16()?;
            if pass_len > 100 {
                return Ok(false);
            }
            if pass_len != 0 {
                let buffer = mfile.read_bytes(pass_len as usize)?;
                let mut password = vec![0u8; 100];
                password[..buffer.len()].copy_from_slice(&buffer);
                self.password = password;
            }
        }

        if self.version_id == 1 {
            return Ok(true);
        }

        self.repeat_mode = mfile.read_int16()?;
        self.repeat_dir = mfile.read_int16()?;
        self.repeat_offset = mfile.read_int32()?;

        if self.version_id >= 3 {
            self.read_tags(mfile)?;
        }

        Ok(true)
    }

    fn read_v4(&mut self, mfile: &mut MifReader) -> Result<bool, String> {
        self.design_name = mfile.read_string()?;
        self.design_file_name = mfile.read_string()?;
        self.design_type = mfile.read_string()?;
        
        // Read username (256 bytes)
        let _username = mfile.read_bytes(256)?;
        
        self.pc = mfile.read_int16()?;
        if self.pc < 0 || self.pc > DESIGN_MAXPARAMETERCOUNT {
            return Ok(false);
        }

        // Read parameters
        for _ in 0..self.pc {
            self.parameters.push(mfile.read_string()?);
        }

        self.read_tags(mfile)?;
        Ok(true)
    }

    fn read(mfile: &mut MifReader) -> Result<Self, String> {
        let mut header = MifHeader {
            file_version: mfile.read_tag(8)?,
            version_id: 0,
            variant_count: 0,
            active_variant: 0,
            flags: 0,
            design_name: String::new(),
            design_file_name: String::new(),
            design_type: String::new(),
            pc: 0,
            parameters: Vec::new(),
            password: Vec::new(),
            repeat_mode: 0,
            repeat_dir: 0,
            repeat_offset: 0,
            tags: Vec::new(),
        };

        header.version_id = header.get_version();
        if header.version_id == -1 {
            return Err("Invalid MIF file version".to_string());
        }

        header.variant_count = mfile.read_int16()?;
        header.active_variant = mfile.read_int16()?;
        header.flags = mfile.read_int16()?;

        if header.version_id <= 0 || header.version_id > 4 {
            return Err("Unsupported MIF file version".to_string());
        }

        let success = if header.version_id <= 3 {
            header.read_v3_and_older(mfile)?
        } else {
            header.read_v4(mfile)?
        };

        if !success {
            return Err("Failed to read MIF header".to_string());
        }

        Ok(header)
    }
}

// Update open_mif function to use the new header implementation
#[tauri::command]
async fn open_mif(request: MifOpenRequest) -> Result<Vec<u8>, String> {
    println!("Opening MIF file: {}", request.file_path);
    
    let mut mif_reader = MifReader::new(&request.file_path)?;
    
    // Read and print first 16 bytes for debugging
    let debug_bytes = mif_reader.read_bytes(16)?;
    println!("First 16 bytes: {:?}", debug_bytes);
    
    // Reset file position
    mif_reader.jump(0)?;
    
    // Read MIF header
    let header = MifHeader::read(&mut mif_reader)?;
    println!("MIF Version: {}, Variants: {}, Active: {}", 
             header.version_id, header.variant_count, header.active_variant);
    println!("Design name: {}", header.design_name);
    
    // Now read the variant header
    let variant_header = match VariantHeaderSection::read(&mut mif_reader) {
        Ok(vh) => vh,
        Err(e) => {
            let pos = mif_reader.stream_position()?;
            println!("Error reading variant header at position {}: {}", pos, e);
            return Err(e);
        }
    };
    
    println!("Variant header tag: '{}'", variant_header.tag);
    println!("Variant version: {}", variant_header.get_variant_version()?);
    println!("Channel counts: {}, {}", variant_header.channel_count1, variant_header.channel_count2);
    
    // Create test image with actual dimensions from the file
    let layer = MifLayer {
        channel_count: variant_header.channel_count1 as u32,
        channels: vec![
            MifChannel {
                width: 400,
                height: 400,
                data: vec![255; 160000], // 400x400 white channel
                name: variant_header.name,
                visibility: true,
                opacity: 1.0,
                color: MifColor {
                    tag: String::new(),
                    red: 255 << 8,
                    green: 255 << 8,
                    blue: 255 << 8,
                    color_type: 1,
                    l: 0,
                    a: 0,
                    b: 0,
                    name: String::new(),
                    description: String::new(),
                    extra_datasize: 0,
                },
            }
        ],
    };
    
    let variant = MifVariant {
        name: "Default".to_string(),
        fabric_color: MifColor {
            tag: String::new(),
            red: 255 << 8,
            green: 255 << 8,
            blue: 255 << 8,
            color_type: 1,
            l: 0,
            a: 0,
            b: 0,
            name: String::new(),
            description: String::new(),
            extra_datasize: 0,
        },
        channel_specs: vec![],
    };
    
    let rgb_image = create_interleaved_rgb(&layer, &variant, 1)?;
    
    let mut output_buffer = Vec::new();
    rgb_image.write_to(&mut Cursor::new(&mut output_buffer), ImageFormat::Png)
        .map_err(|e| format!("Failed to create preview: {}", e))?;
    
    println!("Created preview image of {} bytes", output_buffer.len());
    
    Ok(output_buffer)
}

const IMAGE_DATA_TAG: &str = "DATA001";

#[derive(Debug)]
struct ImageDataSection {
    tag: String,
    width: i32,
    height: i32,
    data_size: i32,
    line_sizes: Vec<i16>,
    line_comps: Vec<i8>,
    data_pointer: u64,
}

// Implement packbits decompression
fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut result = Vec::new();
    let mut i = 0;
    
    while i < data.len() {
        let header = data[i] as i8;
        i += 1;
        
        if header >= 0 {
            // Copy next (header + 1) bytes literally
            let count = (header as usize) + 1;
            if i + count > data.len() {
                return Err("Invalid packbits data: literal count exceeds buffer".to_string());
            }
            result.extend_from_slice(&data[i..i + count]);
            i += count;
        } else if header != -128 {
            // Repeat next byte (-header + 1) times
            if i >= data.len() {
                return Err("Invalid packbits data: no byte to repeat".to_string());
            }
            let count = (-header as usize) + 1;
            let value = data[i];
            result.extend(std::iter::repeat(value).take(count));
            i += 1;
        }
    }
    
    Ok(result)
}

impl ImageDataSection {
    fn read(mfile: &mut MifReader) -> Result<Self, String> {
        let mut section = ImageDataSection {
            tag: String::new(),
            width: 0,
            height: 0,
            data_size: 0,
            line_sizes: Vec::new(),
            line_comps: Vec::new(),
            data_pointer: 0,
        };
        
        section.tag = mfile.read_tag(8)?;
        if section.tag.to_uppercase() != IMAGE_DATA_TAG {
            return Err("Invalid image data tag".to_string());
        }
        
        section.width = mfile.read_int32()?;
        section.height = mfile.read_int32()?;
        section.data_size = mfile.read_int32()?;
        
        // Read line sizes
        for _ in 0..section.height {
            section.line_sizes.push(mfile.read_int16()?);
        }
        
        // Read line compression flags
        for _ in 0..section.height {
            section.line_comps.push(mfile.read_int8()?);
        }
        
        // Store current position as data pointer
        section.data_pointer = mfile.stream_position()?;
        
        // Skip over data section
        mfile.jump(section.data_pointer + section.data_size as u64)?;
        
        Ok(section)
    }
    
    fn read_line(&self, mfile: &mut MifReader, line: i32) -> Result<Vec<u8>, String> {
        let position = if line == 0 {
            self.data_pointer
        } else {
            self.data_pointer + self.line_sizes[..line as usize].iter().map(|&x| x as u64).sum::<u64>()
        };
        
        mfile.jump(position)?;
        let line_buffer = mfile.read_bytes(self.line_sizes[line as usize] as usize)?;
        
        if self.line_comps[line as usize] == 1 {
            decompress(&line_buffer)
        } else {
            Ok(line_buffer)
        }
    }
    
    fn read_data(&self, mfile: &mut MifReader) -> Result<Vec<Vec<u8>>, String> {
        let mut buffer = Vec::with_capacity(self.height as usize);
        for line in 0..self.height {
            buffer.push(self.read_line(mfile, line)?);
        }
        Ok(buffer)
    }

    fn write(&self, writer: &mut MifWriter, data: &[Vec<u8>]) -> Result<(), String> {
        writer.write_tag(&self.tag, 8)?;
        writer.write_int32(self.width)?;
        writer.write_int32(self.height)?;
        
        // Calculate and write data size
        let total_size: i32 = self.line_sizes.iter().map(|&x| x as i32).sum();
        writer.write_int32(total_size)?;
        
        // Write line sizes
        for size in &self.line_sizes {
            writer.write_int16(*size)?;
        }
        
        // Write line compression flags
        for comp in &self.line_comps {
            writer.write_int8(*comp)?;
        }
        
        // Write actual data
        for line in data {
            writer.write_bytes(line)?;
        }
        
        Ok(())
    }
}

// Update MifChannel to use ImageDataSection
impl MifChannel {
    fn read(mfile: &mut MifReader) -> Result<Self, String> {
        let image_data = ImageDataSection::read(mfile)?;
        let data = image_data.read_data(mfile)?;
        
        // Flatten the data into a single vector
        let mut flat_data = Vec::with_capacity((image_data.width * image_data.height) as usize);
        for line in data {
            flat_data.extend(line);
        }
        
        Ok(MifChannel {
            width: image_data.width as u32,
            height: image_data.height as u32,
            data: flat_data,
            name: String::new(), // This should be set from variant spec
            visibility: true,
            opacity: 1.0,
            color: MifColor {
                tag: String::new(),
                red: 0,
                green: 0,
                blue: 0,
                color_type: 0,
                l: 0,
                a: 0,
                b: 0,
                name: String::new(),
                description: String::new(),
                extra_datasize: 0,
            },
        })
    }
}

const IMG_SYS_HEADER_TAG: &str = "MIRACLEIMAGE019";  // size 16
const IMG_SYS_HEADER_TAG_SIZE: usize = 16;

#[derive(Debug)]
struct ImgSysHeaderSection {
    tag: String,
    name: String,
    file_name: String,
    temp_path: String,
    width: u32,
    height: u32,
    cache_count: i16,
    layer_count: i16,
    resolution: u32,
    canvas_color: MifColor,
}

#[derive(Debug)]
struct RepeatTag {
    mode: i16,
    dir: i16,
    offset: i32,
}

#[derive(Debug)]
struct HalftoneTag {
    output_resolution: i32,
    enable: i16,
}

#[derive(Debug)]
struct ChannelOffsets {
    channel_count: i16,
    channel_offsets: Vec<(i16, i16)>,
}

#[derive(Debug)]
struct ImgSysTags {
    rendering_method: Option<i32>,
    channel_offsets: Option<ChannelOffsets>,
    halftone: Option<HalftoneTag>,
    repeat: Option<RepeatTag>,
}

#[derive(Debug)]
struct ImgSysSection {
    header: ImgSysHeaderSection,
    tags: ImgSysTags,
    layers: Vec<MifLayer>,
}

impl ImgSysHeaderSection {
    fn read(mfile: &mut MifReader) -> Result<Self, String> {
        let tag = mfile.read_tag(IMG_SYS_HEADER_TAG_SIZE)?;
        if tag != IMG_SYS_HEADER_TAG {
            return Err("Invalid image system header tag".to_string());
        }

        let header = ImgSysHeaderSection {
            tag,
            name: mfile.read_string()?,
            file_name: mfile.read_string()?,
            temp_path: mfile.read_string()?,
            width: mfile.read_uint32()?,
            height: mfile.read_uint32()?,
            cache_count: mfile.read_int16()?,
            layer_count: 1, // Fixed to 1 as in Python
            resolution: mfile.read_uint32()?,
            canvas_color: MifColor::read(mfile)?,
        };

        Ok(header)
    }
}

fn parse_repeat_tag_data(mfile: &mut MifReader) -> Result<RepeatTag, String> {
    Ok(RepeatTag {
        mode: mfile.read_int16()?,
        dir: mfile.read_int16()?,
        offset: mfile.read_int32()?,
    })
}

fn parse_halftone_tag_data(mfile: &mut MifReader) -> Result<HalftoneTag, String> {
    Ok(HalftoneTag {
        output_resolution: mfile.read_int32()?,
        enable: mfile.read_int16()?,
    })
}

fn parse_channel_offsets_tag_data(mfile: &mut MifReader) -> Result<ChannelOffsets, String> {
    let channel_count = mfile.read_int16()?;
    let mut offsets = Vec::with_capacity(channel_count as usize);
    
    for _ in 0..channel_count {
        let x = mfile.read_int16()?;
        let y = mfile.read_int16()?;
        offsets.push((x, y));
    }
    
    Ok(ChannelOffsets {
        channel_count,
        channel_offsets: offsets,
    })
}

fn parse_rendering_method_tag_data(mfile: &mut MifReader) -> Result<i32, String> {
    mfile.read_int32()
}

fn read_img_sys_tags(mfile: &mut MifReader) -> Result<ImgSysTags, String> {
    let mut tags = ImgSysTags {
        rendering_method: None,
        channel_offsets: None,
        halftone: None,
        repeat: None,
    };

    let mut size = mfile.read_int32()?;
    if size == 0 {
        return Ok(tags);
    }

    loop {
        let tag_id = mfile.read_int16()?;
        match tag_id {
            0 => return Err("Invalid tag id".to_string()),
            1 => tags.repeat = Some(parse_repeat_tag_data(mfile)?),
            2 => tags.halftone = Some(parse_halftone_tag_data(mfile)?),
            3 => tags.channel_offsets = Some(parse_channel_offsets_tag_data(mfile)?),
            4 => tags.rendering_method = Some(parse_rendering_method_tag_data(mfile)?),
            _ => return Ok(ImgSysTags {
                rendering_method: None,
                channel_offsets: None,
                halftone: None,
                repeat: None,
            }),
        }

        size = mfile.read_int32()?;
        if size == 0 {
            break;
        }
    }

    Ok(tags)
}

impl ImgSysSection {
    fn read(mfile: &mut MifReader) -> Result<Self, String> {
        let header = ImgSysHeaderSection::read(mfile)?;
        let tags = read_img_sys_tags(mfile)?;
        
        let layers = Vec::with_capacity(header.layer_count as usize);

        Ok(ImgSysSection {
            header,
            tags,
            layers,
        })
    }

    fn write(&self, writer: &mut MifWriter) -> Result<(), String> {
        // Write header
        writer.write_tag(&self.header.tag, IMG_SYS_HEADER_TAG_SIZE)?;
        writer.write_string(&self.header.name)?;
        writer.write_string(&self.header.file_name)?;
        writer.write_string(&self.header.temp_path)?;
        writer.write_uint32(self.header.width)?;
        writer.write_uint32(self.header.height)?;
        writer.write_int16(self.header.cache_count)?;
        writer.write_int16(self.header.layer_count)?;
        writer.write_uint32(self.header.resolution)?;
        self.header.canvas_color.write(writer)?;
        
        // Write tags
        if let Some(repeat) = &self.tags.repeat {
            writer.write_int32(8)?; // Tag size
            writer.write_int16(1)?; // Tag ID for repeat
            writer.write_int16(repeat.mode)?;
            writer.write_int16(repeat.dir)?;
            writer.write_int32(repeat.offset)?;
        }
        
        if let Some(halftone) = &self.tags.halftone {
            writer.write_int32(6)?; // Tag size
            writer.write_int16(2)?; // Tag ID for halftone
            writer.write_int32(halftone.output_resolution)?;
            writer.write_int16(halftone.enable)?;
        }
        
        if let Some(channel_offsets) = &self.tags.channel_offsets {
            let size = 2 + channel_offsets.channel_count as i32 * 4;
            writer.write_int32(size)?;
            writer.write_int16(3)?; // Tag ID for channel offsets
            writer.write_int16(channel_offsets.channel_count)?;
            for &(x, y) in &channel_offsets.channel_offsets {
                writer.write_int16(x)?;
                writer.write_int16(y)?;
            }
        }
        
        if let Some(rendering_method) = &self.tags.rendering_method {
            writer.write_int32(4)?; // Tag size
            writer.write_int16(4)?; // Tag ID for rendering method
            writer.write_int32(*rendering_method)?;
        }
        
        writer.write_int32(0)?; // End of tags marker
        
        // Write layers
        for _layer in &self.layers {
            // TODO: Implement layer writing once MifLayer write method is added
        }
        
        Ok(())
    }
}

struct MifWriter {
    file: File,
}

impl MifWriter {
    fn new(filepath: &str) -> Result<Self, String> {
        let file = File::create(filepath)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        Ok(MifWriter { file })
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.file.write_all(bytes)
            .map_err(|e| format!("Failed to write bytes: {}", e))
    }

    fn write_int8(&mut self, value: i8) -> Result<(), String> {
        self.file.write_all(&[value as u8])
            .map_err(|e| format!("Failed to write i8: {}", e))
    }

    fn write_int16(&mut self, value: i16) -> Result<(), String> {
        self.file.write_all(&value.to_le_bytes())
            .map_err(|e| format!("Failed to write i16: {}", e))
    }

    fn write_int32(&mut self, value: i32) -> Result<(), String> {
        self.file.write_all(&value.to_le_bytes())
            .map_err(|e| format!("Failed to write i32: {}", e))
    }

    fn write_uint16(&mut self, value: u16) -> Result<(), String> {
        self.file.write_all(&value.to_le_bytes())
            .map_err(|e| format!("Failed to write u16: {}", e))
    }

    fn write_uint32(&mut self, value: u32) -> Result<(), String> {
        self.file.write_all(&value.to_le_bytes())
            .map_err(|e| format!("Failed to write u32: {}", e))
    }

    fn write_float(&mut self, value: f32) -> Result<(), String> {
        self.file.write_all(&value.to_le_bytes())
            .map_err(|e| format!("Failed to write f32: {}", e))
    }

    fn write_boolean(&mut self, value: bool) -> Result<(), String> {
        self.write_bytes(&[if value { 1 } else { 0 }])
    }

    fn write_tag(&mut self, tag: &str, size: usize) -> Result<(), String> {
        let mut buffer = vec![0u8; size];
        let bytes = tag.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        self.write_bytes(&buffer)
    }

    fn write_string(&mut self, value: &str) -> Result<(), String> {
        let bytes = value.as_bytes();
        self.write_int16(bytes.len() as i16)?;
        self.write_bytes(bytes)
    }
}

impl MifHeader {
    fn write(&self, writer: &mut MifWriter) -> Result<(), String> {
        writer.write_tag(&self.file_version, 8)?;
        writer.write_int16(self.variant_count)?;
        writer.write_int16(self.active_variant)?;
        writer.write_int16(self.flags)?;
        
        if self.version_id <= 3 {
            writer.write_string(&self.design_name)?;
            writer.write_string(&self.design_file_name)?;
            writer.write_string(&self.design_type)?;
            
            if self.version_id != 1 {
                writer.write_int16(self.pc)?;
            }
            
            for param in &self.parameters {
                writer.write_string(param)?;
            }
            
            if self.version_id != 4 {
                writer.write_int16(self.password.len() as i16)?;
                if !self.password.is_empty() {
                    writer.write_bytes(&self.password)?;
                }
            }
            
            if self.version_id > 1 {
                self.write_tags(writer)?;
            }
        } else {
            writer.write_string(&self.design_name)?;
            writer.write_string(&self.design_file_name)?;
            writer.write_string(&self.design_type)?;
            
            // Write 256 bytes of username (empty in this case)
            writer.write_bytes(&vec![0u8; 256])?;
            
            writer.write_int16(self.pc)?;
            for param in &self.parameters {
                writer.write_string(param)?;
            }
            
            self.write_tags(writer)?;
        }
        
        Ok(())
    }
    
    fn write_tags(&self, writer: &mut MifWriter) -> Result<(), String> {
        for tag in &self.tags {
            writer.write_int32(tag.size)?;
            writer.write_int16(tag.id)?;
            writer.write_bytes(&tag.data)?;
        }
        writer.write_int32(0)?; // End of tags marker
        Ok(())
    }
}

#[tauri::command]
async fn read_mif_variants(request: MifOpenRequest) -> Result<Vec<Vec<u8>>, String> {
    println!("Reading variants from MIF file: {}", request.file_path);
    
    let mut mif_reader = MifReader::new(&request.file_path)?;
    
    // Read MIF header first
    let header = MifHeader::read(&mut mif_reader)?;
    println!("Found {} variants", header.variant_count);
    
    let mut variant_bytes = Vec::new();
    
    // Read each variant
    for variant_index in 0..header.variant_count {
        println!("Reading variant {}", variant_index);
        
        // Store the start position of this variant
        let variant_start = mif_reader.stream_position()?;
        
        // Read variant header
        let variant_header = VariantHeaderSection::read(&mut mif_reader)?;
        println!("Variant {}: '{}' with {} channels", 
                variant_index, 
                variant_header.name, 
                variant_header.channel_count1);
        
        // Read channel specs
        for _ in 0..variant_header.channel_count1 {
            // Skip channel specs for now, we just want the raw bytes
            let _channel_spec = MifChannelSpec {
                name: mif_reader.read_string()?,
                visibility: mif_reader.read_boolean()?,
                opacity: mif_reader.read_float()?,
                color: MifColor::read(&mut mif_reader)?,
            };
        }
        
        // Read preview data
        let preview_size = mif_reader.read_int32()?;
        let _preview_data = mif_reader.read_bytes(preview_size as usize)?;
        
        // Read simulation properties if version > 1
        if variant_header.get_variant_version()? > 1 {
            // Skip simulation properties tag
            let _sim_props_tag = mif_reader.read_tag(8)?;
            
            // Skip name and fabric name
            let _name = mif_reader.read_string()?;
            let _fabric_name = mif_reader.read_string()?;
            
            // Skip fabric color
            let _fabric_color = MifColor::read(&mut mif_reader)?;
            
            // Skip linear values
            let _linear1 = mif_reader.read_int32()?;
            let _linear2 = mif_reader.read_int32()?;
            let _linear3 = mif_reader.read_int32()?;
            
            // Skip forced colors and discharged print
            let _forced_colors = mif_reader.read_int32()?;
            let _discharged_print = mif_reader.read_int32()?;
            
            // Skip data blocks
            let _data1 = mif_reader.read_bytes(12)?;
            let _data2 = mif_reader.read_bytes(12)?;
            let _data3 = mif_reader.read_bytes(12)?;
            
            // Skip extra data size
            let _extra_data_size = mif_reader.read_int32()?;
        }
        
        // Read parameters if version 4
        if variant_header.get_variant_version()? == 4 {
            let pc = mif_reader.read_int16()?;
            for _ in 0..pc {
                let _param = mif_reader.read_string()?;
            }
        }
        
        // Store the end position of this variant
        let variant_end = mif_reader.stream_position()?;
        
        // Jump back to start of variant and read all bytes
        mif_reader.jump(variant_start)?;
        let variant_size = (variant_end - variant_start) as usize;
        let variant_data = mif_reader.read_bytes(variant_size)?;
        variant_bytes.push(variant_data);
        
        // Jump to end of variant for next iteration
        mif_reader.jump(variant_end)?;
    }
    
    println!("Successfully read {} variants", variant_bytes.len());
    Ok(variant_bytes)
}

#[derive(Debug, Serialize)]
struct VariantInfo {
    index: i16,
    name: String,
    description: String,
    channel_count: i16,
    preview_data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct SelectVariantRequest {
    file_path: String,
    variant_index: i16,
}

#[tauri::command]
async fn list_variants(request: MifOpenRequest) -> Result<Vec<VariantInfo>, String> {
    println!("Listing variants from MIF file: {}", request.file_path);
    
    let mut mif_reader = MifReader::new(&request.file_path)?;
    
    // Read MIF header first
    let header = MifHeader::read(&mut mif_reader)?;
    println!("Found {} variants", header.variant_count);
    
    let mut variants = Vec::new();
    
    // Read each variant
    for variant_index in 0..header.variant_count {
        println!("Reading variant {}", variant_index);
        
        // Read variant header
        let variant_header = VariantHeaderSection::read(&mut mif_reader)?;
        let version = variant_header.get_variant_version()?;
        
        // Read channel specs (skip for now)
        for _ in 0..variant_header.channel_count1 {
            let _channel_spec = MifChannelSpec {
                name: mif_reader.read_string()?,
                visibility: mif_reader.read_boolean()?,
                opacity: mif_reader.read_float()?,
                color: MifColor::read(&mut mif_reader)?,
            };
        }
        
        // Read preview data
        let preview_size = mif_reader.read_int32()?;
        let preview_data = mif_reader.read_bytes(preview_size as usize)?;
        
        // Create variant info
        let variant_info = VariantInfo {
            index: variant_index,
            name: variant_header.name.clone(),
            description: variant_header.description.clone(),
            channel_count: variant_header.channel_count1,
            preview_data,
        };
        
        variants.push(variant_info);
        
        // Skip rest of variant data
        if version > 1 {
            // Skip simulation properties
            let _sim_props_tag = mif_reader.read_tag(8)?;
            let _name = mif_reader.read_string()?;
            let _fabric_name = mif_reader.read_string()?;
            let _fabric_color = MifColor::read(&mut mif_reader)?;
            let _linear1 = mif_reader.read_int32()?;
            let _linear2 = mif_reader.read_int32()?;
            let _linear3 = mif_reader.read_int32()?;
            let _forced_colors = mif_reader.read_int32()?;
            let _discharged_print = mif_reader.read_int32()?;
            let _data1 = mif_reader.read_bytes(12)?;
            let _data2 = mif_reader.read_bytes(12)?;
            let _data3 = mif_reader.read_bytes(12)?;
            let _extra_data_size = mif_reader.read_int32()?;
        }
        
        if version == 4 {
            let pc = mif_reader.read_int16()?;
            for _ in 0..pc {
                let _param = mif_reader.read_string()?;
            }
        }
    }
    
    Ok(variants)
}

#[tauri::command]
async fn select_variant(request: SelectVariantRequest) -> Result<Vec<u8>, String> {
    println!("Selecting variant {} from file {}", request.variant_index, request.file_path);
    
    let mut mif_reader = MifReader::new(&request.file_path)?;
    
    // Read MIF header
    let header = MifHeader::read(&mut mif_reader)?;
    if request.variant_index >= header.variant_count {
        return Err(format!("Invalid variant index: {}", request.variant_index));
    }
    
    // Skip to the requested variant
    for _ in 0..request.variant_index {
        let vh = VariantHeaderSection::read(&mut mif_reader)?;
        
        // Skip channel specs
        for _ in 0..vh.channel_count1 {
            let _name = mif_reader.read_string()?;
            let _visibility = mif_reader.read_boolean()?;
            let _opacity = mif_reader.read_float()?;
            let _color = MifColor::read(&mut mif_reader)?;
        }
        
        // Skip preview
        let preview_size = mif_reader.read_int32()?;
        let _preview = mif_reader.read_bytes(preview_size as usize)?;
        
        // Skip simulation properties if needed
        if vh.get_variant_version()? > 1 {
            let _sim_props_tag = mif_reader.read_tag(8)?;
            let _name = mif_reader.read_string()?;
            let _fabric_name = mif_reader.read_string()?;
            let _fabric_color = MifColor::read(&mut mif_reader)?;
            let _linear1 = mif_reader.read_int32()?;
            let _linear2 = mif_reader.read_int32()?;
            let _linear3 = mif_reader.read_int32()?;
            let _forced_colors = mif_reader.read_int32()?;
            let _discharged_print = mif_reader.read_int32()?;
            let _data1 = mif_reader.read_bytes(12)?;
            let _data2 = mif_reader.read_bytes(12)?;
            let _data3 = mif_reader.read_bytes(12)?;
            let _extra_data_size = mif_reader.read_int32()?;
        }
        
        if vh.get_variant_version()? == 4 {
            let pc = mif_reader.read_int16()?;
            for _ in 0..pc {
                let _param = mif_reader.read_string()?;
            }
        }
    }
    
    // Read the requested variant
    let variant_header = VariantHeaderSection::read(&mut mif_reader)?;
    println!("Loading variant: {}", variant_header.name);
    
    // Create test image with actual dimensions
    let layer = MifLayer {
        channel_count: variant_header.channel_count1 as u32,
        channels: vec![
            MifChannel {
                width: 400,
                height: 400,
                data: vec![255; 160000], // 400x400 white channel
                name: variant_header.name,
                visibility: true,
                opacity: 1.0,
                color: MifColor {
                    tag: String::new(),
                    red: 255 << 8,
                    green: 255 << 8,
                    blue: 255 << 8,
                    color_type: 1,
                    l: 0,
                    a: 0,
                    b: 0,
                    name: String::new(),
                    description: String::new(),
                    extra_datasize: 0,
                },
            }
        ],
    };
    
    let variant = MifVariant {
        name: "Default".to_string(),
        fabric_color: MifColor {
            tag: String::new(),
            red: 255 << 8,
            green: 255 << 8,
            blue: 255 << 8,
            color_type: 1,
            l: 0,
            a: 0,
            b: 0,
            name: String::new(),
            description: String::new(),
            extra_datasize: 0,
        },
        channel_specs: vec![],
    };
    
    let rgb_image = create_interleaved_rgb(&layer, &variant, 1)?;
    
    let mut output_buffer = Vec::new();
    rgb_image.write_to(&mut Cursor::new(&mut output_buffer), ImageFormat::Png)
        .map_err(|e| format!("Failed to create preview: {}", e))?;
    
    println!("Created preview image of {} bytes", output_buffer.len());
    Ok(output_buffer)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("Starting application...");
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            process_image,
            rotate_image,
            open_mif,
            process_mif_image,
            read_mif_variants,
            list_variants,
            select_variant
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
