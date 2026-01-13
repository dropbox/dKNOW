#!/usr/bin/env python3
"""Download brand logos from alternative sources for logo detection testing

Since Wikimedia Commons blocks automated downloads, this script uses alternative
approaches to obtain logo images for testing:
1. Creates simple synthetic logos using PIL (for testing the CLIP infrastructure)
2. Downloads from free logo repositories when available
3. Provides instructions for manual download
"""

import os
import json
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFont
    PIL_AVAILABLE = True
except ImportError:
    PIL_AVAILABLE = False
    print("Warning: PIL not available. Install with: pip3 install pillow")


# Logo categories and brands
LOGOS = {
    'tech': ['Apple', 'Google', 'Microsoft', 'Amazon', 'Meta', 'Netflix',
             'Tesla', 'Nvidia', 'Intel', 'AMD', 'Samsung', 'Sony',
             'IBM', 'Oracle', 'Adobe', 'Spotify', 'Twitter', 'LinkedIn'],
    'sportswear': ['Nike', 'Adidas', 'Puma', 'Under_Armour', 'Reebok',
                   'New_Balance', 'Converse', 'Vans', 'Asics', 'Fila'],
    'food': ['Coca-Cola', 'Pepsi', 'McDonalds', 'Starbucks', 'Burger_King',
             'KFC', 'Subway', 'Pizza_Hut', 'Taco_Bell', 'Red_Bull'],
    'automotive': ['Tesla', 'BMW', 'Mercedes', 'Audi', 'Toyota', 'Honda',
                   'Ford', 'Chevrolet', 'Volkswagen', 'Ferrari'],
    'retail': ['Walmart', 'Target', 'Amazon', 'Costco', 'IKEA',
               'Home_Depot', 'Best_Buy', 'Walgreens', 'CVS'],
    'fashion': ['Gucci', 'Chanel', 'Louis_Vuitton', 'Prada', 'Versace',
                'Burberry', 'Ralph_Lauren', 'Calvin_Klein'],
    'airlines': ['United', 'Delta', 'American', 'Southwest',
                 'British_Airways', 'Lufthansa', 'Emirates']
}

# Brand colors (approximate)
BRAND_COLORS = {
    'Apple': '#A2AAAD',
    'Google': '#4285F4',
    'Microsoft': '#00A4EF',
    'Amazon': '#FF9900',
    'Meta': '#0668E1',
    'Netflix': '#E50914',
    'Nike': '#000000',
    'Adidas': '#000000',
    'Coca-Cola': '#F40009',
    'Pepsi': '#004B93',
    'McDonalds': '#FFC72C',
    'Starbucks': '#00704A',
    'Tesla': '#CC0000',
    'BMW': '#1C69D4',
    'Mercedes': '#00ADEF',
    'Walmart': '#0071CE',
    'Target': '#CC0000',
}


def create_synthetic_logo(brand_name, category, output_path):
    """Create a simple synthetic logo for testing"""
    if not PIL_AVAILABLE:
        return False

    try:
        # Create image with brand color or default
        color = BRAND_COLORS.get(brand_name, '#333333')
        img = Image.new('RGB', (400, 400), color='white')
        draw = ImageDraw.Draw(img)

        # Draw a colored rectangle (simple logo substitute)
        margin = 50
        draw.rectangle([margin, margin, 350, 350], fill=color)

        # Add brand name text
        try:
            font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 40)
        except:
            font = ImageFont.load_default()

        # Get text bbox for centering
        text = brand_name.replace('_', ' ')
        bbox = draw.textbbox((0, 0), text, font=font)
        text_width = bbox[2] - bbox[0]
        text_height = bbox[3] - bbox[1]

        text_x = (400 - text_width) // 2
        text_y = (400 - text_height) // 2

        # Draw text in white
        draw.text((text_x, text_y), text, fill='white', font=font)

        # Save as PNG
        img.save(output_path)
        return True

    except Exception as e:
        print(f"  Error creating synthetic logo: {e}")
        return False


def download_from_clearbit(brand_name, output_path):
    """Try to download logo from Clearbit Logo API (free tier)"""
    import requests

    # Clearbit provides free logos via simple URL scheme
    # Format: https://logo.clearbit.com/{domain}

    # Common domain mappings
    domain_map = {
        'Apple': 'apple.com',
        'Google': 'google.com',
        'Microsoft': 'microsoft.com',
        'Amazon': 'amazon.com',
        'Meta': 'meta.com',
        'Netflix': 'netflix.com',
        'Tesla': 'tesla.com',
        'Nike': 'nike.com',
        'Adidas': 'adidas.com',
        'Coca-Cola': 'coca-cola.com',
        'Pepsi': 'pepsi.com',
        'McDonalds': 'mcdonalds.com',
        'Starbucks': 'starbucks.com',
    }

    domain = domain_map.get(brand_name, f"{brand_name.lower().replace('_', '')}.com")
    url = f"https://logo.clearbit.com/{domain}?size=200"

    try:
        session = requests.Session()
        session.headers.update({'User-Agent': 'Mozilla/5.0'})
        response = session.get(url, timeout=10)

        if response.status_code == 200 and len(response.content) > 100:
            with open(output_path, 'wb') as f:
                f.write(response.content)
            return True
    except:
        pass

    return False


def main():
    """Main execution"""
    output_dir = Path("models/logo-detection/clip_database/logos")

    # Create output directories
    for category in LOGOS.keys():
        (output_dir / category).mkdir(parents=True, exist_ok=True)

    results = {
        'synthetic': [],
        'clearbit': [],
        'failed': []
    }

    print("Logo Download Strategy:")
    print("1. Try Clearbit Logo API (free tier)")
    print("2. Create synthetic logos for testing CLIP infrastructure")
    print()

    total_brands = sum(len(brands) for brands in LOGOS.values())
    print(f"Processing {total_brands} brand logos\n")

    for category, brands in LOGOS.items():
        print(f"\n{'='*60}")
        print(f"Category: {category.upper()} ({len(brands)} brands)")
        print(f"{'='*60}\n")

        for brand in brands:
            brand_safe = brand.replace('_', ' ')
            print(f"Processing {brand_safe}...")

            category_dir = output_dir / category
            output_png = category_dir / f"{brand}.png"

            # Check if already exists
            if output_png.exists():
                print(f"  ✓ Already exists: {output_png.name}")
                continue

            # Try Clearbit first
            if download_from_clearbit(brand, output_png):
                print(f"  ✓ Downloaded from Clearbit: {output_png.name}")
                file_size = output_png.stat().st_size
                results['clearbit'].append({
                    'brand': brand_safe,
                    'category': category,
                    'filename': output_png.name,
                    'size': file_size
                })
                continue

            # Fall back to synthetic logo
            if create_synthetic_logo(brand, category, output_png):
                print(f"  ✓ Created synthetic logo: {output_png.name}")
                file_size = output_png.stat().st_size
                results['synthetic'].append({
                    'brand': brand_safe,
                    'category': category,
                    'filename': output_png.name,
                    'size': file_size
                })
            else:
                print(f"  ✗ Failed to create logo for {brand_safe}")
                results['failed'].append(brand_safe)

    # Save results
    log_path = output_dir / 'download_log.json'
    with open(log_path, 'w') as f:
        json.dump(results, f, indent=2)

    # Print summary
    print(f"\n{'='*60}")
    print("DOWNLOAD SUMMARY")
    print(f"{'='*60}")
    print(f"Clearbit downloads: {len(results['clearbit'])} logos")
    print(f"Synthetic logos:    {len(results['synthetic'])} logos")
    print(f"Failed:             {len(results['failed'])} logos")
    print(f"Total attempted:    {total_brands} brands")

    success_count = len(results['clearbit']) + len(results['synthetic'])
    print(f"\nSuccess rate: {success_count}/{total_brands} ({100*success_count/total_brands:.1f}%)")
    print(f"\nResults log saved to: {log_path}")

    print(f"\n{'='*60}")
    print("NEXT STEPS")
    print(f"{'='*60}")
    print("1. Run: cargo run -p build_logo_database")
    print("2. This will generate CLIP embeddings for all downloaded logos")
    print("3. Logo detection will work with these logos")
    print()
    print("Note: Synthetic logos are simplified shapes for testing.")
    print("For production use, obtain real brand logos with proper licensing.")


if __name__ == '__main__':
    main()
