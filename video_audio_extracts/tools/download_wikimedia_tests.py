#!/usr/bin/env python3
"""
Download real media files from Wikimedia Commons for test matrix.
Usage: python3 tools/download_wikimedia_tests.py <feature> <format> [count] [category]
"""

import requests
import json
import os
import sys
from pathlib import Path
from urllib.parse import quote


class WikimediaDownloader:
    API_URL = "https://commons.wikimedia.org/w/api.php"

    def __init__(self, output_dir="test_files_wikimedia"):
        self.output_dir = Path(output_dir)
        self.session = requests.Session()
        self.session.headers.update(
            {
                "User-Agent": "VideoAudioExtractTestSuite/1.0 (Research/Testing; ayates@dropbox.com)"
            }
        )

    def search_by_category(self, category, limit=50):
        """Search files in a Wikimedia Commons category"""
        params = {
            "action": "query",
            "list": "categorymembers",
            "cmtitle": f"Category:{category}",
            "cmtype": "file",
            "cmlimit": limit,
            "format": "json",
        }

        try:
            response = self.session.get(self.API_URL, params=params, timeout=30)
            response.raise_for_status()
            data = response.json()

            members = data.get("query", {}).get("categorymembers", [])

            # Get detailed info for each file
            file_infos = []
            for member in members:
                info = self.get_file_info(member["title"])
                if info:
                    file_infos.append(info)

            return file_infos
        except Exception as e:
            print(f"Error searching category {category}: {e}")
            return []

    def get_file_info(self, title):
        """Get file URL and metadata"""
        params = {
            "action": "query",
            "titles": title,
            "prop": "imageinfo",
            "iiprop": "url|size|mime|mediatype",
            "format": "json",
        }

        try:
            response = self.session.get(self.API_URL, params=params, timeout=30)
            data = response.json()

            pages = data.get("query", {}).get("pages", {})
            for page in pages.values():
                if "imageinfo" in page and page["imageinfo"]:
                    info = page["imageinfo"][0]
                    return {
                        "title": title,
                        "url": info.get("url"),
                        "size": info.get("size", 0),
                        "mime": info.get("mime", ""),
                        "mediatype": info.get("mediatype", ""),
                    }
        except Exception as e:
            print(f"Error getting info for {title}: {e}")

        return None

    def download_file(self, url, output_path):
        """Download file from URL"""
        output_path.parent.mkdir(parents=True, exist_ok=True)

        try:
            print(f"  Downloading {output_path.name}...")
            response = self.session.get(url, stream=True, timeout=120)
            response.raise_for_status()

            with open(output_path, "wb") as f:
                for chunk in response.iter_content(chunk_size=8192):
                    f.write(chunk)

            return True
        except Exception as e:
            print(f"  Error: {e}")
            return False

    def get_extension_from_mime(self, mime):
        """Get file extension from MIME type"""
        mime_map = {
            "video/mp4": "mp4",
            "video/quicktime": "mov",
            "video/x-matroska": "mkv",
            "video/webm": "webm",
            "video/x-msvideo": "avi",
            "video/ogg": "ogv",
            "audio/wav": "wav",
            "audio/x-wav": "wav",
            "audio/mpeg": "mp3",
            "audio/mp3": "mp3",
            "audio/flac": "flac",
            "audio/x-flac": "flac",
            "audio/mp4": "m4a",
            "audio/x-m4a": "m4a",
            "audio/ogg": "ogg",
            "image/jpeg": "jpg",
            "image/png": "png",
            "image/webp": "webp",
            "image/gif": "gif",
        }
        return mime_map.get(mime.lower(), None)

    def download_test_matrix_files(self, feature, format_ext, count=5, category=None):
        """Download test files for a (feature, format) cell"""
        print(f"\n{'='*60}")
        print(f"Downloading: ({feature}, {format_ext})")
        print(f"{'='*60}")

        if not category:
            # Use default categories based on format
            category = self.get_default_category(format_ext)

        print(f"Category: {category}")

        # Search for files
        files = self.search_by_category(category, limit=count * 5)

        if not files:
            print(f"No files found in category '{category}'")
            return []

        # Filter by exact format match AND size constraint (<100MB for GitHub)
        MAX_SIZE = 99_000_000  # 99MB (under GitHub 100MB limit)
        filtered = []
        for f in files:
            if not f.get("url") or not f.get("mime"):
                continue

            actual_ext = self.get_extension_from_mime(f["mime"])
            file_size = f.get("size", 0)

            # Filter by format AND size
            if actual_ext == format_ext.lower() and file_size <= MAX_SIZE:
                filtered.append(f)

        if not filtered:
            print(f"No {format_ext} files <100MB found in category '{category}'")
            print(f"  (Searched {len(files)} files with various formats)")
            return []

        # Take first N
        selected = filtered[:count]
        print(f"Found {len(selected)}/{count} {format_ext} files (<100MB)")

        # Download
        downloaded = []
        for i, file_info in enumerate(selected, 1):
            # Get actual extension from mime type
            actual_ext = self.get_extension_from_mime(file_info["mime"])

            # Clean filename
            filename = file_info["title"].replace("File:", "").replace(" ", "_")
            # Remove existing extension if present
            for ext in [
                ".mp4",
                ".mov",
                ".mkv",
                ".webm",
                ".wav",
                ".mp3",
                ".jpg",
                ".png",
            ]:
                if filename.lower().endswith(ext):
                    filename = filename[: -len(ext)]
                    break

            filename = f"{i:02d}_{filename}.{actual_ext}"
            output_path = self.output_dir / actual_ext / feature / filename

            if self.download_file(file_info["url"], output_path):
                downloaded.append(
                    {
                        "path": str(output_path),
                        "size": file_info["size"],
                        "url": file_info["url"],
                        "title": file_info["title"],
                        "mime": file_info["mime"],
                        "format": actual_ext,
                    }
                )

        # Save metadata
        if downloaded:
            metadata_path = self.output_dir / actual_ext / feature / "metadata.json"
            with open(metadata_path, "w") as f:
                json.dump(downloaded, f, indent=2)

        print(f"Downloaded {len(downloaded)}/{count} files")
        return downloaded

    def get_default_category(self, format_ext):
        """Get default Wikimedia category for format"""
        categories = {
            "mp4": "Videos",
            "mov": "QuickTime videos",
            "mkv": "Matroska videos",
            "webm": "WebM videos",
            "wav": "Audio files in WAV format",
            "mp3": "Audio files in MP3 format",
            "flac": "Audio files in FLAC format",
            "m4a": "Audio files in M4A format",
            "jpg": "JPEG files",
            "jpeg": "JPEG files",
            "png": "PNG files",
        }
        return categories.get(format_ext.lower(), "Videos")


# Main execution
if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(
            "Usage: python3 tools/download_wikimedia_tests.py <feature> <format> [count] [category]"
        )
        print(
            "Example: python3 tools/download_wikimedia_tests.py transcription mp4 5 'Speeches'"
        )
        sys.exit(1)

    feature = sys.argv[1]
    format_ext = sys.argv[2]
    count = int(sys.argv[3]) if len(sys.argv) > 3 else 5
    category = sys.argv[4] if len(sys.argv) > 4 else None

    downloader = WikimediaDownloader()
    downloaded = downloader.download_test_matrix_files(
        feature, format_ext, count, category
    )

    print(f"\n{'='*60}")
    print(f"Summary: Downloaded {len(downloaded)} files for ({feature}, {format_ext})")
    print(f"{'='*60}")
