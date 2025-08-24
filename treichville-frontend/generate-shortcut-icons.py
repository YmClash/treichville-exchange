from PIL import Image, ImageDraw, ImageFont
import os

# Create icons for PWA shortcuts
def create_shortcut_icon(text, filename, bg_color=(255, 107, 0)):
    size = 96
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Background with rounded corners
    corner_radius = size // 5
    # Create rounded rectangle
    for y in range(size):
        for x in range(size):
            # Check if pixel is within rounded rectangle
            in_rect = True
            if x < corner_radius and y < corner_radius:
                # Top-left corner
                if (x - corner_radius) ** 2 + (y - corner_radius) ** 2 > corner_radius ** 2:
                    in_rect = False
            elif x > size - corner_radius and y < corner_radius:
                # Top-right corner
                if (x - (size - corner_radius)) ** 2 + (y - corner_radius) ** 2 > corner_radius ** 2:
                    in_rect = False
            elif x < corner_radius and y > size - corner_radius:
                # Bottom-left corner
                if (x - corner_radius) ** 2 + (y - (size - corner_radius)) ** 2 > corner_radius ** 2:
                    in_rect = False
            elif x > size - corner_radius and y > size - corner_radius:
                # Bottom-right corner
                if (x - (size - corner_radius)) ** 2 + (y - (size - corner_radius)) ** 2 > corner_radius ** 2:
                    in_rect = False
            
            if in_rect:
                draw.point((x, y), fill=bg_color + (255,))
    
    # Draw symbol
    try:
        font_size = 48
        font = ImageFont.truetype("C:/Windows/Fonts/Arial.ttf", font_size)
    except:
        try:
            font = ImageFont.truetype("/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf", font_size)
        except:
            font = ImageFont.load_default()
    
    # Get text bounding box
    bbox = draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    
    # Calculate position to center text
    x = (size - text_width) // 2
    y = (size - text_height) // 2 - 2  # Slight adjustment
    
    # Draw white text
    draw.text((x, y), text, fill=(255, 255, 255, 255), font=font)
    
    # Save the icon
    filepath = os.path.join('public/icons', filename)
    img.save(filepath, 'PNG')
    print(f"Created: {filepath}")

# Generate shortcut icons
print("Generating PWA shortcut icons...")

# Exchange icon (arrows)
create_shortcut_icon("⇄", "exchange.png", (255, 107, 0))

# Wallet icon (dollar sign)
create_shortcut_icon("$", "wallet.png", (46, 125, 50))

# Also create status icons that service worker references
create_shortcut_icon("✓", "checkmark.png", (46, 125, 50))
create_shortcut_icon("✕", "xmark.png", (220, 53, 69))

print("\nAll shortcut icons generated successfully!")