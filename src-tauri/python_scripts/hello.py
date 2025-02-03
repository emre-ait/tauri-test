import subprocess
import sys
import os
import cv2
import numpy as np
import base64
from PIL import Image
import io
import time

print("Python script loaded!")
print(f"Python path: {sys.path}")
print(f"Current directory: {os.getcwd()}")

def resize_if_needed(img, max_dimension=1024):
    """Resize image if any dimension is larger than max_dimension"""
    height, width = img.shape[:2]
    
    # Calculate aspect ratio
    if max(height, width) > max_dimension:
        if height > width:
            new_height = max_dimension
            new_width = int(width * (max_dimension / height))
        else:
            new_width = max_dimension
            new_height = int(height * (max_dimension / width))
        
        img = cv2.resize(img, (new_width, new_height), interpolation=cv2.INTER_AREA)
        return img, True
    return img, False

def process_image(image_base64: str) -> str:
    """Process image and add markers"""
    try:
        start_time = time.time()
        processing_info = []

        # Decode base64 image
        try:
            # First convert base64 to PIL Image for safer handling
            image_data = base64.b64decode(image_base64.split(',')[1] if ',' in image_base64 else image_base64)
            pil_image = Image.open(io.BytesIO(image_data))
            
            # Convert to RGB if necessary
            if pil_image.mode != 'RGB':
                pil_image = pil_image.convert('RGB')
            
            # Get original dimensions
            original_width, original_height = pil_image.size
            processing_info.append(f"Original size: {original_width}x{original_height}")
            
            # Pre-resize with PIL if image is very large
            if max(original_width, original_height) > 1024:
                ratio = 1024 / max(original_width, original_height)
                new_width = int(original_width * ratio)
                new_height = int(original_height * ratio)
                pil_image = pil_image.resize((new_width, new_height), Image.Resampling.LANCZOS)
                processing_info.append(f"Resized to: {new_width}x{new_height}")
            
            # Convert PIL image to OpenCV format
            img_array = np.array(pil_image)
            img = cv2.cvtColor(img_array, cv2.COLOR_RGB2BGR)
            
        except Exception as e:
            print(f"Error during image loading/conversion: {e}")
            return f"Error: Failed to load image - {str(e)}"
        
        # Add a red circle in the center
        height, width = img.shape[:2]
        center = (width // 2, height // 2)
        circle_radius = min(50, width//10)
        cv2.circle(img, center, circle_radius, (0, 0, 255), max(1, width//500))
        
        # Add processing info
        font_scale = max(0.5, width/1000)
        thickness = max(1, int(width/500))
        y_position = 30
        for i, info in enumerate(processing_info):
            cv2.putText(img, info, (30, y_position + i * 30), 
                       cv2.FONT_HERSHEY_SIMPLEX, font_scale, (255, 0, 0), thickness)

        # Calculate processing time
        processing_time = time.time() - start_time
        cv2.putText(img, f"Processing time: {processing_time:.3f}s", 
                   (30, y_position + len(processing_info) * 30), 
                   cv2.FONT_HERSHEY_SIMPLEX, font_scale, (255, 0, 0), thickness)
        
        # Convert back to base64 with error handling
        try:
            success, buffer = cv2.imencode('.jpg', img, [cv2.IMWRITE_JPEG_QUALITY, 85])
            if not success:
                return "Error: Failed to encode processed image"
            img_base64 = base64.b64encode(buffer).decode('utf-8')
            return f"data:image/jpeg;base64,{img_base64}"
        except Exception as e:
            print(f"Error during image encoding: {e}")
            return f"Error: Failed to encode image - {str(e)}"
            
    except Exception as e:
        print(f"Error processing image: {e}")
        return f"Error: {str(e)}"

def show_notification(title: str, message: str):
    """Show a system notification"""
    try:
        print(f"Trying to send notification: {title} - {message}")
        subprocess.run(['notify-send', title, message])
        print("Notification sent successfully")
    except Exception as e:
        print(f"Failed to send notification: {e}")

def process_file(file_path: str) -> str:
    """Just show a notification and return success"""
    print("\n" + "="*50)
    print("Python: process_file called!")
    
    show_notification("Hello", "This is a test notification from Python!")
    
    return "success"

def calculate(operation: str, a: float, b: float) -> str:
    """Perform basic calculator operations"""
    try:
        result = 0
        if operation == "add":
            result = a + b
        elif operation == "subtract":
            result = a - b
        elif operation == "multiply":
            result = a * b
        elif operation == "divide":
            if b == 0:
                return "Error: Division by zero"
            result = a / b
        else:
            return f"Error: Unknown operation {operation}"
        
        return f"{result}"
    except Exception as e:
        return f"Error: {str(e)}"

def show_alert():
    return "Hello from Python!" 