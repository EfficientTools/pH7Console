#!/usr/bin/env python3
from PIL import Image, ImageDraw, ImageFont
import os

def create_terminal_icon(size=512):
    # Create a new image with transparent background
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Colors
    bg_color = (26, 27, 38, 255)  # Dark terminal background
    border_color = (80, 250, 123, 255)  # Green accent (AI theme)
    text_color = (248, 248, 242, 255)  # Light text
    cursor_color = (80, 250, 123, 255)  # Green cursor
    
    # Draw terminal window background
    margin = size // 10
    terminal_rect = [margin, margin, size - margin, size - margin]
    draw.rounded_rectangle(terminal_rect, radius=size//20, fill=bg_color, outline=border_color, width=3)
    
    # Draw title bar
    title_bar_height = size // 8
    title_bar_rect = [margin + 3, margin + 3, size - margin - 3, margin + 3 + title_bar_height]
    draw.rounded_rectangle(title_bar_rect, radius=size//30, fill=(40, 42, 54, 255))
    
    # Draw window controls (close, minimize, maximize)
    control_size = size // 30
    control_y = margin + title_bar_height // 2
    
    # Close button (red)
    draw.ellipse([margin + 20, control_y - control_size//2, margin + 20 + control_size, control_y + control_size//2], fill=(255, 85, 85, 255))
    # Minimize button (yellow)
    draw.ellipse([margin + 20 + control_size * 2, control_y - control_size//2, margin + 20 + control_size * 3, control_y + control_size//2], fill=(255, 184, 108, 255))
    # Maximize button (green)
    draw.ellipse([margin + 20 + control_size * 4, control_y - control_size//2, margin + 20 + control_size * 5, control_y + control_size//2], fill=(80, 250, 123, 255))
    
    # Draw terminal content area
    content_y = margin + title_bar_height + 10
    line_height = size // 20
    
    # Draw prompt lines to simulate terminal
    prompt_text = "$ "
    command_text = "pH7Console --ai"
    
    # Calculate font size
    font_size = max(12, size // 25)
    
    try:
        # Try to use a monospace font
        font = ImageFont.truetype("/System/Library/Fonts/Monaco.ttc", font_size)
    except:
        try:
            font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", font_size)
        except:
            font = ImageFont.load_default()
    
    # Draw terminal prompt
    y_pos = content_y
    draw.text((margin + 20, y_pos), prompt_text, fill=cursor_color, font=font)
    
    # Get text width to position command
    try:
        prompt_width = font.getlength(prompt_text)
    except:
        prompt_width = len(prompt_text) * font_size * 0.6
    
    draw.text((margin + 20 + prompt_width, y_pos), command_text, fill=text_color, font=font)
    
    # Draw cursor
    try:
        total_width = font.getlength(prompt_text + command_text)
    except:
        total_width = len(prompt_text + command_text) * font_size * 0.6
    
    cursor_x = margin + 20 + total_width + 5
    draw.rectangle([cursor_x, y_pos, cursor_x + 2, y_pos + font_size], fill=cursor_color)
    
    # Draw AI indicator
    ai_y = y_pos + line_height + 10
    ai_text = "🤖 AI Ready"
    draw.text((margin + 20, ai_y), ai_text, fill=cursor_color, font=font)
    
    # Draw pH7 logo/text
    logo_y = ai_y + line_height + 10
    logo_text = "pH7"
    try:
        logo_font = ImageFont.truetype("/System/Library/Fonts/Monaco.ttc", font_size + 4)
    except:
        logo_font = font
    
    draw.text((margin + 20, logo_y), logo_text, fill=border_color, font=logo_font)
    
    return img

def main():
    # Create icons in different sizes
    sizes = [16, 32, 64, 128, 256, 512]
    
    for size in sizes:
        icon = create_terminal_icon(size)
        
        # Save PNG
        icon.save(f'icon_{size}x{size}.png', 'PNG')
        
        # Create specific files needed by Tauri
        if size == 32:
            icon.save('32x32.png', 'PNG')
        elif size == 128:
            icon.save('128x128.png', 'PNG')
            icon.save('128x128@2x.png', 'PNG')
        elif size == 512:
            icon.save('icon.png', 'PNG')
    
    print("Icons created successfully!")
    print("Files created:")
    for f in os.listdir('.'):
        if f.endswith('.png'):
            print(f"  {f}")

if __name__ == "__main__":
    main()
