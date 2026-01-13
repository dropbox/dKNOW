#!/usr/bin/env python3
"""Download top 100 brand logos from Wikimedia Commons

This script downloads logos from Wikimedia Commons for logo detection testing.
It uses the Wikimedia API to search for logos and downloads the highest resolution versions.
"""

import requests
import os
import json
import time
from pathlib import Path
from urllib.parse import unquote

# Top 100+ brand logos to download, organized by category
LOGOS = {
    'tech': [
        'Apple Inc', 'Google', 'Microsoft', 'Amazon', 'Meta Platforms', 'Netflix',
        'Tesla Inc', 'Nvidia', 'Intel', 'AMD', 'Samsung', 'Sony',
        'IBM', 'Oracle Corporation', 'Adobe Inc', 'Spotify', 'Twitter', 'LinkedIn',
        'Slack', 'Dropbox', 'Zoom', 'GitHub', 'GitLab', 'Reddit'
    ],
    'sportswear': [
        'Nike', 'Adidas', 'Puma', 'Under Armour', 'Reebok',
        'New Balance', 'Converse', 'Vans', 'Asics', 'Fila',
        'Lululemon', 'Columbia Sportswear', 'The North Face', 'Patagonia'
    ],
    'food': [
        'Coca-Cola', 'Pepsi', 'McDonald\'s', 'Starbucks', 'Burger King',
        'KFC', 'Subway', 'Dunkin\' Donuts', 'Pizza Hut', 'Taco Bell',
        'Red Bull', 'Monster Energy', 'Nestle', 'Heinz', 'Kraft Foods',
        'General Mills', 'Kellogg\'s', 'Chipotle', 'Domino\'s Pizza'
    ],
    'automotive': [
        'Tesla Inc', 'BMW', 'Mercedes-Benz', 'Audi', 'Toyota', 'Honda',
        'Ford Motor Company', 'Chevrolet', 'Volkswagen', 'Hyundai', 'Ferrari',
        'Porsche', 'Lamborghini', 'Mazda', 'Subaru', 'Nissan', 'Lexus'
    ],
    'retail': [
        'Walmart', 'Target Corporation', 'Amazon', 'Costco', 'IKEA',
        'Home Depot', 'Best Buy', 'Walgreens', 'CVS Pharmacy',
        'Whole Foods', '7-Eleven', 'eBay', 'Etsy', 'Wayfair'
    ],
    'fashion': [
        'Gucci', 'Chanel', 'Louis Vuitton', 'Prada', 'Versace',
        'Burberry', 'Ralph Lauren', 'Calvin Klein', 'Tommy Hilfiger',
        'Zara', 'H&M', 'Gap Inc', 'Uniqlo', 'Levi Strauss'
    ],
    'airlines': [
        'United Airlines', 'Delta Air Lines', 'American Airlines', 'Southwest Airlines',
        'British Airways', 'Lufthansa', 'Emirates', 'Air France',
        'Singapore Airlines', 'Qantas', 'JetBlue'
    ]
}


def wikimedia_search(brand_name, category, session):
    """Search Wikimedia Commons for brand logo"""
    url = "https://commons.wikimedia.org/w/api.php"

    # Try multiple search terms for better results
    search_terms = [
        f'{brand_name} logo',
        f'{brand_name} logo.svg',
        f'Logo {brand_name}',
    ]

    for search_term in search_terms:
        params = {
            'action': 'query',
            'format': 'json',
            'list': 'search',
            'srsearch': search_term,
            'srnamespace': 6,  # File namespace
            'srlimit': 10,
            'srprop': 'size'
        }

        try:
            response = session.get(url, params=params, timeout=10)
            response.raise_for_status()
            data = response.json()
            results = data.get('query', {}).get('search', [])

            if results:
                return results
        except Exception as e:
            print(f"  Search error for '{search_term}': {e}")
            continue

    return []


def get_image_url(filename, session):
    """Get direct download URL for a Wikimedia Commons file"""
    url = "https://commons.wikimedia.org/w/api.php"
    params = {
        'action': 'query',
        'format': 'json',
        'titles': f'File:{filename}',
        'prop': 'imageinfo',
        'iiprop': 'url'
    }

    try:
        response = session.get(url, params=params, timeout=10)
        response.raise_for_status()
        data = response.json()

        pages = data.get('query', {}).get('pages', {})
        for page_id, page_data in pages.items():
            imageinfo = page_data.get('imageinfo', [])
            if imageinfo:
                return imageinfo[0].get('url')
    except Exception as e:
        print(f"  Error getting image URL: {e}")
        return None

    return None


def download_file(url, output_path, session):
    """Download file from URL to output path"""
    try:
        response = session.get(url, timeout=30, stream=True)
        response.raise_for_status()

        with open(output_path, 'wb') as f:
            for chunk in response.iter_content(chunk_size=8192):
                f.write(chunk)

        return True
    except Exception as e:
        print(f"  Download error: {e}")
        return False


def download_logo(category, brand_name, output_dir, results_log, session):
    """Download logo for a specific brand"""
    print(f"Downloading {brand_name} logo...")

    # Check if already downloaded
    category_dir = output_dir / category
    existing_files = list(category_dir.glob(f"{brand_name.replace('/', '_').replace(' ', '_')}.*"))
    if existing_files:
        print(f"  ✓ Already exists: {existing_files[0].name}")
        results_log['already_exists'].append(brand_name)
        return True

    # Search Wikimedia Commons
    results = wikimedia_search(brand_name, category, session)

    if not results:
        print(f"  ✗ No results found for {brand_name}")
        results_log['not_found'].append(brand_name)
        return False

    # Try each result until we successfully download one
    for result in results:
        title = result['title']
        filename = title.replace('File:', '')

        # Skip if filename doesn't look like a logo
        filename_lower = filename.lower()
        if not any(keyword in filename_lower for keyword in ['logo', brand_name.lower().split()[0]]):
            continue

        print(f"  Trying: {filename}")

        # Get direct download URL
        image_url = get_image_url(filename, session)
        if not image_url:
            continue

        # Determine file extension
        ext = Path(unquote(image_url)).suffix
        if not ext or ext == '.':
            ext = '.png'  # Default to PNG

        # Clean brand name for filename
        safe_brand_name = brand_name.replace('/', '_').replace(' ', '_').replace("'", '')
        output_path = category_dir / f"{safe_brand_name}{ext}"

        # Download the file
        if download_file(image_url, output_path, session):
            file_size = output_path.stat().st_size
            print(f"  ✓ Downloaded: {output_path.name} ({file_size:,} bytes)")
            results_log['downloaded'].append({
                'brand': brand_name,
                'category': category,
                'filename': output_path.name,
                'size': file_size,
                'source_url': image_url
            })
            return True

    print(f"  ✗ Failed to download any suitable logo for {brand_name}")
    results_log['failed'].append(brand_name)
    return False


def main():
    """Main execution"""
    output_dir = Path("models/logo-detection/clip_database/logos")

    # Create output directories
    for category in LOGOS.keys():
        (output_dir / category).mkdir(parents=True, exist_ok=True)

    # Create session with proper User-Agent
    session = requests.Session()
    session.headers.update({
        'User-Agent': 'VideoExtractLogoDownloader/1.0 (https://github.com/ayates_dbx/video_audio_extracts; research@example.com) requests/2.0'
    })

    # Results tracking
    results_log = {
        'downloaded': [],
        'already_exists': [],
        'not_found': [],
        'failed': []
    }

    total_brands = sum(len(brands) for brands in LOGOS.values())
    print(f"Attempting to download {total_brands} brand logos from Wikimedia Commons\n")

    # Download logos by category
    for category, brands in LOGOS.items():
        print(f"\n{'='*60}")
        print(f"Category: {category.upper()} ({len(brands)} brands)")
        print(f"{'='*60}\n")

        for brand in brands:
            download_logo(category, brand, output_dir, results_log, session)
            time.sleep(1.0)  # Be nice to Wikimedia servers

    # Save results log
    log_path = output_dir / 'download_log.json'
    with open(log_path, 'w') as f:
        json.dump(results_log, f, indent=2)

    # Print summary
    print(f"\n{'='*60}")
    print("DOWNLOAD SUMMARY")
    print(f"{'='*60}")
    print(f"Downloaded:      {len(results_log['downloaded'])} logos")
    print(f"Already exists:  {len(results_log['already_exists'])} logos")
    print(f"Not found:       {len(results_log['not_found'])} logos")
    print(f"Failed:          {len(results_log['failed'])} logos")
    print(f"Total attempted: {total_brands} brands")

    success_count = len(results_log['downloaded']) + len(results_log['already_exists'])
    print(f"\nSuccess rate: {success_count}/{total_brands} ({100*success_count/total_brands:.1f}%)")
    print(f"\nResults log saved to: {log_path}")

    # Print failed brands for manual review
    if results_log['not_found'] or results_log['failed']:
        print(f"\n{'='*60}")
        print("BRANDS THAT NEED MANUAL REVIEW")
        print(f"{'='*60}")
        for brand in results_log['not_found'] + results_log['failed']:
            print(f"  - {brand}")


if __name__ == '__main__':
    main()
