from PIL import Image, ImageDraw, ImageFont
import os

# Create icons directory if it doesn't exist
os.makedirs('public/icons', exist_ok=True)

# Define sizes for different icon types
sizes = [
    (72, 'badge-72x72.png'),
    (96, 'icon-96x96.png'),
    (128, 'icon-128x128.png'),
    (144, 'icon-144x144.png'),
    (152, 'icon-152x152.png'),
    (192, 'icon-192x192.png'),
    (384, 'icon-384x384.png'),
    (512, 'icon-512x512.png'),
]

# Additional sizes for Apple devices
apple_sizes = [
    (120, 'apple-touch-icon-120x120.png'),
    (180, 'apple-touch-icon-180x180.png'),
]

# Favicon sizes
favicon_sizes = [
    (16, 'favicon-16x16.png'),
    (32, 'favicon-32x32.png'),
]

def create_gradient_icon(size, filename):
    # Create a new image with RGBA mode for transparency
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Create rounded rectangle background with gradient effect
    # We'll simulate gradient by drawing multiple rectangles
    for i in range(size):
        # Calculate color interpolation from orange to green
        ratio = i / size
        # Orange RGB (255, 107, 0) to Green RGB (46, 125, 50)
        r = int(255 * (1 - ratio) + 46 * ratio)
        g = int(107 * (1 - ratio) + 125 * ratio)
        b = int(0 * (1 - ratio) + 50 * ratio)
        
        # Draw horizontal line for gradient effect
        draw.rectangle([(0, i), (size, i+1)], fill=(r, g, b, 255))
    
    # Create a circular mask for rounded icon
    mask = Image.new('L', (size, size), 0)
    mask_draw = ImageDraw.Draw(mask)
    
    # Draw rounded rectangle (circle for app icon)
    corner_radius = size // 5
    mask_draw.rounded_rectangle(
        [(0, 0), (size-1, size-1)],
        radius=corner_radius,
        fill=255
    )
    
    # Apply mask to create rounded corners
    img.putalpha(mask)
    
    # Draw text "TE" in the center
    text = "TE"
    
    # Try to use a font, fallback to default if not available
    try:
        # Calculate font size relative to icon size
        font_size = int(size * 0.4)
        # Try to use a system font
        from PIL import ImageFont
        try:
            # Try Windows font path
            font = ImageFont.truetype("C:/Windows/Fonts/Arial.ttf", font_size)
        except:
            try:
                # Try Linux/Mac font path
                font = ImageFont.truetype("/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf", font_size)
            except:
                # Use default font
                font = ImageFont.load_default()
    except:
        font = ImageFont.load_default()
    
    # Create a new layer for text
    text_layer = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    text_draw = ImageDraw.Draw(text_layer)
    
    # Get text bounding box
    bbox = text_draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    
    # Calculate position to center text
    x = (size - text_width) // 2
    y = (size - text_height) // 2
    
    # Draw white text
    text_draw.text((x, y), text, fill=(255, 255, 255, 255), font=font)
    
    # Composite text layer over gradient background
    img = Image.alpha_composite(img, text_layer)
    
    # Save the icon
    filepath = os.path.join('public/icons', filename)
    img.save(filepath, 'PNG')
    print(f"Created: {filepath}")

# Generate all icons
print("Generating PWA icons...")

# Standard PWA icons
for size, filename in sizes:
    create_gradient_icon(size, filename)

# Apple touch icons
for size, filename in apple_sizes:
    create_gradient_icon(size, filename)

# Favicons
for size, filename in favicon_sizes:
    create_gradient_icon(size, filename)

# Create main favicon.ico with multiple sizes
print("\nCreating favicon.ico...")
favicon_16 = Image.open('public/icons/favicon-16x16.png')
favicon_32 = Image.open('public/icons/favicon-32x32.png')
favicon_16.save('public/favicon.ico', format='ICO', sizes=[(16, 16), (32, 32)])
print("Created: public/favicon.ico")

# Create maskable icon (with safe area padding)
print("\nCreating maskable icon...")
maskable = Image.new('RGBA', (512, 512), (255, 107, 0, 255))
draw = ImageDraw.Draw(maskable)

# Create gradient background
for i in range(512):
    ratio = i / 512
    r = int(255 * (1 - ratio) + 46 * ratio)
    g = int(107 * (1 - ratio) + 125 * ratio)
    b = int(0 * (1 - ratio) + 50 * ratio)
    draw.rectangle([(0, i), (512, i+1)], fill=(r, g, b, 255))

# Add safe area (80% of icon)
safe_area = int(512 * 0.8)
offset = (512 - safe_area) // 2

# Draw text in safe area
text = "TE"
try:
    font = ImageFont.truetype("C:/Windows/Fonts/Arial.ttf", 150)
except:
    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf", 150)
    except:
        font = ImageFont.load_default()

text_layer = Image.new('RGBA', (512, 512), (0, 0, 0, 0))
text_draw = ImageDraw.Draw(text_layer)
bbox = text_draw.textbbox((0, 0), text, font=font)
text_width = bbox[2] - bbox[0]
text_height = bbox[3] - bbox[1]
x = (512 - text_width) // 2
y = (512 - text_height) // 2
text_draw.text((x, y), text, fill=(255, 255, 255, 255), font=font)
maskable = Image.alpha_composite(maskable, text_layer)
maskable.save('public/icons/icon-maskable-512x512.png', 'PNG')
print("Created: public/icons/icon-maskable-512x512.png")

print("\n✅ All icons generated successfully!")
print("\nNext steps:")
print("1. Icons are ready in public/icons/")
print("2. Favicon.ico is in public/")
print("3. Update manifest.json if needed")
print("4. Test PWA installation on mobile devices")